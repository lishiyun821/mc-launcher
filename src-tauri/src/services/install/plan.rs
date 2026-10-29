use std::path::{Path, PathBuf};

use crate::core::models::Library;
use crate::infra::http::{Downloader, MIRROR_BASE};
use crate::infra::task::TaskHandle;

use super::rules;
use super::version_json::ResolvedVersion;

/// 一个待下载文件
#[derive(Debug, Clone)]
pub struct PlannedFile {
    /// 主源 URL
    pub url: String,
    /// 镜像回退 URL（None 表示没有）
    pub mirror_url: Option<String>,
    /// 落地绝对路径
    pub dest: PathBuf,
    /// 相对 game 目录的路径（日志/调试用）
    pub relative: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    /// 显示名（如 `org.lwjgl:lwjgl:3.3.3`）
    pub name: String,
}

/// 资源对象（assets）
#[derive(Debug, Clone)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

/// 一次安装要下的全部东西
#[derive(Debug, Default)]
pub struct DownloadPlan {
    pub client: Option<PlannedFile>,
    pub libraries: Vec<PlannedFile>,
    pub natives: Vec<PlannedFile>,
    pub assets_index: Option<PlannedFile>,
    pub assets: Vec<AssetObject>,
}

impl DownloadPlan {
    /// 需要下载的文件总数（不含 assets，单独计）
    pub fn file_count(&self) -> usize {
        self.libraries.len() + self.natives.len() + self.assets_index.is_some() as usize
    }

    /// 预估总字节数
    pub fn total_bytes(&self) -> u64 {
        self.libraries
            .iter()
            .chain(self.natives.iter())
            .chain(self.assets_index.iter())
            .filter_map(|f| f.size)
            .sum::<u64>()
            + self.assets.iter().map(|a| a.size).sum::<u64>()
    }
}

/// 计算「把这个版本装齐」需要下载哪些东西（纯计算，不下载）
pub async fn build_plan(
    dl: &Downloader,
    mc_dir: &Path,
    rv: &ResolvedVersion,
    task: &TaskHandle,
) -> Result<DownloadPlan, anyhow::Error> {
    let libs_dir = mc_dir.join("libraries");
    let assets_dir = mc_dir.join("assets");
    let mut plan = DownloadPlan::default();

    /* ── ① libraries ── */
    task.progress(0.03, "计算库文件清单").await;

    for lib in &rv.libraries {
        if !rules::allows(&lib.rules) {
            continue;
        }

        // 新格式：native 是独立条目（name 带 classifier）
        let is_native_entry = lib
            .name
            .split(':')
            .nth(3)
            .map(|cls| cls.starts_with("natives-"))
            .unwrap_or(false);

        let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) else {
            // 老版本有些库没有 artifact，靠 classifiers 单独给
            continue;
        };

        let Some(rel) = artifact.path.clone().or_else(|| maven_path(&lib.name)) else {
            tracing::warn!("库 {} 既没有 path 也无法从坐标推导路径，跳过", lib.name);
            continue;
        };

        let file = PlannedFile {
            // 国内环境镜像优先，官方兜底
            url: format!("{MIRROR_BASE}/maven/{rel}"),
            mirror_url: Some(artifact.url.clone()),
            dest: libs_dir.join(&rel),
            relative: rel.clone(),
            sha1: artifact.sha1.clone(),
            size: artifact.size,
            name: lib.name.clone(),
        };

        if is_native_entry {
            plan.natives.push(file);
        } else {
            plan.libraries.push(file);
        }
    }

    // 老格式 native：classifiers 字典里的（≤1.18）
    for (lib, classifier) in native_classifier_entries(&rv.libraries) {
        let Some(downloads) = lib.downloads.as_ref() else {
            continue;
        };
        let Some(artifact) = downloads
            .classifiers
            .as_ref()
            .and_then(|c| c.get(&classifier))
        else {
            continue;
        };
        let Some(rel) = artifact.path.clone() else {
            continue;
        };
        plan.natives.push(PlannedFile {
            url: format!("{MIRROR_BASE}/maven/{rel}"),
            mirror_url: Some(artifact.url.clone()),
            dest: libs_dir.join(&rel),
            relative: rel,
            sha1: artifact.sha1.clone(),
            size: artifact.size,
            name: lib.name.clone(),
        });
    }

    /* ── ② 客户端 jar ── */
    let client_rel = format!("versions/{}/{}.jar", rv.id, rv.id);
    plan.client = Some(PlannedFile {
        url: rv.client.url.clone(),
        mirror_url: None,
        dest: mc_dir.join(&client_rel),
        relative: client_rel,
        sha1: rv.client.sha1.clone(),
        size: rv.client.size,
        name: format!("{} 客户端", rv.id),
    });

    /* ── ③ assets 索引 ── */
    task.progress(0.06, "获取资源索引").await;

    let index_rel = format!("assets/indexes/{}.json", rv.asset_index.id);
    plan.assets_index = Some(PlannedFile {
        url: format!(
            "{MIRROR_BASE}/assets/indexes/{}.json",
            rv.asset_index.id
        ),
        mirror_url: Some(rv.asset_index.url.clone()),
        dest: assets_dir.join("indexes").join(format!("{}.json", rv.asset_index.id)),
        relative: index_rel,
        sha1: rv.asset_index.sha1.clone(),
        size: rv.asset_index.size,
        name: format!("assets index {}", rv.asset_index.id),
    });

    // 资源索引内容可能已经在缓存里，先尝试本地读，避免每次都要下载
    let cached_index = assets_dir
        .join("indexes")
        .join(format!("{}.json", rv.asset_index.id));

    let index_bytes: Option<Vec<u8>> = if cached_index.is_file() {
        let ok = match rv.asset_index.sha1.as_deref() {
            Some(expected) => crate::infra::downloader::file_sha1(&cached_index)
                .map(|h| h.eq_ignore_ascii_case(expected))
                .unwrap_or(false),
            None => true,
        };
        if ok {
            std::fs::read(&cached_index).ok()
        } else {
            None
        }
    } else {
        None
    };

    let index_bytes = match index_bytes {
        Some(b) => b,
        None => dl.get_bytes(&rv.asset_index.url).await?,
    };

    plan.assets = parse_asset_objects(&index_bytes)?;
    tracing::info!(
        "资源索引 {} 共 {} 个对象，合计 {:.1} MB",
        rv.asset_index.id,
        plan.assets.len(),
        plan.assets.iter().map(|a| a.size as f64).sum::<f64>() / 1024.0 / 1024.0
    );

    tracing::info!(
        "下载清单：库 {} 个 + native {} 个 + assets {} 个，合计 {:.1} MB",
        plan.libraries.len(),
        plan.natives.len(),
        plan.assets.len(),
        plan.total_bytes() as f64 / 1024.0 / 1024.0
    );

    Ok(plan)
}

/// 从 assets 索引 JSON 里取出所有对象
fn parse_asset_objects(bytes: &[u8]) -> Result<Vec<AssetObject>, anyhow::Error> {
    #[derive(serde::Deserialize)]
    struct Index {
        #[serde(default)]
        objects: std::collections::HashMap<String, Obj>,
    }
    #[derive(serde::Deserialize)]
    struct Obj {
        hash: String,
        size: u64,
    }

    let idx: Index =
        serde_json::from_slice(bytes).map_err(|e| anyhow::anyhow!("资源索引解析失败: {e}"))?;

    Ok(idx
        .objects
        .into_values()
        .map(|o| AssetObject {
            hash: o.hash,
            size: o.size,
        })
        .collect())
}

/// 老格式 natives：从 `natives` 字段里取当前平台的 classifier
fn native_classifier_entries(libs: &[Library]) -> Vec<(Library, String)> {
    libs.iter()
        .filter(|l| rules::allows(&l.rules))
        .filter_map(|l| {
            l.natives
                .as_ref()
                .and_then(|n| n.get(rules::os_name()))
                .map(|cls| (l.clone(), cls.clone()))
        })
        .collect()
}

/// 从 Maven 坐标推导相对路径：group:artifact:version[:classifier]
/// → group/artifact/version/artifact-version[-classifier].jar
fn maven_path(coords: &str) -> Option<String> {
    let base = coords.split('@').next().unwrap_or(coords);
    let parts: Vec<&str> = base.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let classifier = parts.get(3).map(|c| format!("-{c}")).unwrap_or_default();
    let ext = coords.split('@').nth(1).unwrap_or("jar");
    Some(format!(
        "{}/{artifact}/{version}/{artifact}-{version}{classifier}.{ext}",
        group.replace('.', "/")
    ))
}

/// assets 对象的落地路径
pub fn asset_path(assets_dir: &Path, hash: &str) -> PathBuf {
    let prefix = &hash[..2.min(hash.len())];
    assets_dir.join("objects").join(prefix).join(hash)
}

/// 供外部判断 native 解压目录
pub fn natives_dir(mc_dir: &Path, version: &str) -> PathBuf {
    mc_dir.join("versions").join(version).join("natives")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_path_basic() {
        assert_eq!(
            maven_path("org.ow2.asm:asm:9.7.1").as_deref(),
            Some("org/ow2/asm/asm/9.7.1/asm-9.7.1.jar")
        );
    }

    #[test]
    fn maven_path_with_classifier() {
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-windows").as_deref(),
            Some("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar")
        );
    }

    #[test]
    fn asset_path_layout() {
        let p = asset_path(Path::new("C:/mc"), "abcdef1234");
        assert!(p.to_string_lossy().contains("objects\\ab\\abcdef1234"));
    }
}
