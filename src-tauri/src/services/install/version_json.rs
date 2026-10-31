use std::path::{Path, PathBuf};

use crate::core::models::*;
use crate::infra::http::Downloader;
use crate::infra::task::TaskHandle;

use super::rules;

/// 解析 + 合并后的完整版本定义，可以直接拿去算下载清单 / 拼启动参数
#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    pub id: String,
    pub kind: Option<String>,
    pub main_class: String,
    pub libraries: Vec<Library>,
    pub asset_index: AssetIndex,
    pub client: Artifact,
    pub java_major: Option<u32>,
    pub arguments: Arguments,
    pub minecraft_arguments: Option<String>,
    pub release_time: Option<String>,
    /// 继承链，从最顶层父到子，如 ["1.21", "1.21.11"]
    pub chain: Vec<String>,
}

impl ResolvedVersion {
    /// 当前平台真正需要加载的 libraries（按 rules 过滤）
    pub fn platform_libraries(&self) -> Vec<Library> {
        self.libraries
            .iter()
            .filter(|l| rules::allows(&l.rules))
            .cloned()
            .collect()
    }

    /// 需要下载的 native 库
    ///
    /// 三代格式都要认：
    /// - 老（≤1.18）：`natives: { "windows": "natives-windows" }` 指向同库的 classifier
    /// - 中（1.19~1.20）：`downloads.classifiers["natives-windows"]`
    /// - 新（1.21+）：native 是独立 library 条目，name 自带 classifier，
    ///   如 `org.lwjgl:lwjgl:3.3.3:natives-windows`
    pub fn native_classifiers(&self) -> Vec<(String, String)> {
        let os_key = format!("natives-{}", rules::os_name());

        self.libraries
            .iter()
            .filter(|l| rules::allows(&l.rules))
            .filter_map(|l| {
                // 新格式：name 第 4 段是 natives-*
                if let Some(cls) = classifier_from_name(&l.name) {
                    if cls.starts_with("natives-") {
                        return Some((l.name.clone(), cls.to_string()));
                    }
                }
                // 老格式：natives 字段
                if let Some(cls) = l.natives.as_ref().and_then(|n| n.get(rules::os_name())) {
                    return Some((l.name.clone(), cls.clone()));
                }
                // 中格式：classifiers 字典
                l.downloads
                    .as_ref()
                    .and_then(|d| d.classifiers.as_ref())
                    .and_then(|c| c.get(&os_key))
                    .map(|_| (l.name.clone(), os_key.clone()))
            })
            .collect()
    }
}

/// 解析一个版本，并把 `inheritsFrom` 递归合并成完整定义。
///
/// 做三件事：
/// 1. 逐层下载 `<id>.json`，原始内容存为 `versions/<id>/<id>.raw.json`
/// 2. 沿 inheritsFrom 向上爬，收集整条链（父 → 子）
/// 3. 合并后写回 `versions/<id>/<id>.json`
pub async fn resolve(
    dl: &Downloader,
    versions_dir: &Path,
    manifest: &VersionManifest,
    id: &str,
    task: &TaskHandle,
) -> Result<ResolvedVersion, anyhow::Error> {
    let mut chain: Vec<VersionJson> = Vec::new();
    let mut cur = id.to_string();

    loop {
        task.check_cancel()?;

        // 两种来源：
        // ① 官方清单里有的（原版、快照）—— 走网络拉
        // ② 清单里没有的（Fabric / Quilt / Forge 这类第三方版本）—— 安装器已经把
        //    原始定义写进 versions/<id>/<id>.raw.json，直接读本地
        let bytes = match manifest.versions.iter().find(|v| v.id == cur) {
            Some(v) => {
                task.progress(0.05, format!("获取 {cur} 的版本定义")).await;
                dl.get_bytes(&v.url).await?
            }
            None => {
                let dir = versions_dir.join(&cur);
                let raw = dir.join(format!("{cur}.raw.json"));
                let merged_file = dir.join(format!("{cur}.json"));
                if raw.is_file() {
                    task.progress(0.05, format!("读取本地版本 {cur}")).await;
                    std::fs::read(&raw)?
                } else if merged_file.is_file() {
                    task.progress(0.05, format!("读取本地版本 {cur}")).await;
                    std::fs::read(&merged_file)?
                } else {
                    return Err(anyhow::anyhow!(
                        "版本清单和本地都找不到 {cur}（第三方加载器请先用对应的安装命令）"
                    ));
                }
            }
        };

        let dir = versions_dir.join(&cur);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(format!("{cur}.raw.json")), &bytes)?;

        let vj: VersionJson = serde_json::from_slice(&bytes)
            .map_err(|e| anyhow::anyhow!("{cur}.json 解析失败: {e}"))?;

        let parent = vj.inherits_from.clone();
        chain.push(vj);

        match parent {
            Some(p) if !p.is_empty() => {
                if chain.len() >= 8 {
                    return Err(anyhow::anyhow!("inheritsFrom 链过深，疑似循环: {id}"));
                }
                cur = p;
            }
            _ => break,
        }
    }

    chain.reverse(); // 父在前、子在后
    let names: Vec<String> = chain.iter().map(|v| v.id.clone()).collect();
    tracing::info!("版本 {id} 继承链: {names:?}");

    let merged = merge(chain, id)?;
    save_resolved(versions_dir, &merged)?;
    Ok(merged)
}

/// 把整条链合并成一份
fn merge(chain: Vec<VersionJson>, id: &str) -> Result<ResolvedVersion, anyhow::Error> {
    let main_class = chain
        .iter()
        .rev()
        .find_map(|v| v.main_class.clone())
        .ok_or_else(|| anyhow::anyhow!("版本 {id} 缺少 mainClass（继承链也不提供）"))?;

    let asset_index = chain
        .iter()
        .rev()
        .find_map(|v| v.asset_index.clone())
        .ok_or_else(|| anyhow::anyhow!("版本 {id} 缺少 assetIndex"))?;

    let client = chain
        .iter()
        .rev()
        .find_map(|v| v.downloads.as_ref().and_then(|d| d.client.clone()))
        .ok_or_else(|| anyhow::anyhow!("版本 {id} 缺少客户端 jar 信息（downloads.client）"))?;

    // libraries：按 group:artifact[:classifier] 去重，子版本覆盖父版本
    let mut libraries: Vec<Library> = Vec::new();
    for vj in &chain {
        for lib in &vj.libraries {
            let key = library_key(&lib.name);
            match libraries.iter().position(|l| library_key(&l.name) == key) {
                Some(i) => libraries[i] = lib.clone(),
                None => libraries.push(lib.clone()),
            }
        }
    }

    // arguments：父在前、子在后依次拼接
    let mut game: Vec<serde_json::Value> = Vec::new();
    let mut jvm: Vec<serde_json::Value> = Vec::new();
    for vj in &chain {
        if let Some(a) = &vj.arguments {
            game.extend(a.game.iter().cloned());
            jvm.extend(a.jvm.iter().cloned());
        }
    }

    let kind = chain.iter().rev().find_map(|v| v.kind.clone());
    let release_time = chain.iter().rev().find_map(|v| v.release_time.clone());
    let java_major = chain
        .iter()
        .rev()
        .find_map(|v| v.java_version.as_ref().and_then(|j| j.major_version));
    let minecraft_arguments = chain
        .iter()
        .rev()
        .find_map(|v| v.minecraft_arguments.clone());

    Ok(ResolvedVersion {
        id: id.to_string(),
        kind,
        main_class,
        libraries,
        asset_index,
        client,
        java_major,
        arguments: Arguments { game, jvm },
        minecraft_arguments,
        release_time,
        chain: chain.iter().map(|v| v.id.clone()).collect(),
    })
}

/// 把合并结果写回 `versions/<id>/<id>.json`，后续启动直接读这一份
fn save_resolved(versions_dir: &Path, rv: &ResolvedVersion) -> Result<PathBuf, anyhow::Error> {
    let dir = versions_dir.join(&rv.id);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", rv.id));

    let merged = VersionJson {
        id: rv.id.clone(),
        kind: rv.kind.clone(),
        main_class: Some(rv.main_class.clone()),
        inherits_from: None, // 合并后不再继承
        merged_chain: Some(rv.chain.clone()),
        minecraft_arguments: rv.minecraft_arguments.clone(),
        arguments: Some(rv.arguments.clone()),
        libraries: rv.libraries.clone(),
        asset_index: Some(rv.asset_index.clone()),
        assets: None,
        downloads: Some(Downloads {
            client: Some(rv.client.clone()),
            server: None,
        }),
        java_version: rv.java_major.map(|m| JavaVersion {
            component: None,
            major_version: Some(m),
        }),
        release_time: rv.release_time.clone(),
        time: None,
    };

    let text = serde_json::to_string_pretty(&merged)?;
    std::fs::write(&path, text)?;
    tracing::info!("合并后的版本定义已保存: {}", path.display());
    Ok(path)
}

/// 从合并后的版本 JSON 重建（启动时读磁盘上的 <id>.json）
pub fn from_merged(json: &VersionJson) -> Result<ResolvedVersion, anyhow::Error> {
    let main_class = json
        .main_class
        .clone()
        .ok_or_else(|| anyhow::anyhow!("版本定义缺少 mainClass"))?;
    let asset_index = json
        .asset_index
        .clone()
        .ok_or_else(|| anyhow::anyhow!("版本定义缺少 assetIndex"))?;
    let client = json
        .downloads
        .as_ref()
        .and_then(|d| d.client.clone())
        .ok_or_else(|| anyhow::anyhow!("版本定义缺少客户端 jar 信息"))?;

    Ok(ResolvedVersion {
        id: json.id.clone(),
        kind: json.kind.clone(),
        main_class,
        libraries: json.libraries.clone(),
        asset_index,
        client,
        java_major: json.java_version.as_ref().and_then(|j| j.major_version),
        arguments: json.arguments.clone().unwrap_or(Arguments {
            game: Vec::new(),
            jvm: Vec::new(),
        }),
        minecraft_arguments: json.minecraft_arguments.clone(),
        release_time: json.release_time.clone(),
        chain: json
            .merged_chain
            .clone()
            .unwrap_or_else(|| vec![json.id.clone()]),
    })
}

/// Maven 坐标去重键：`group:artifact`（带 classifier 时保留 classifier）
fn library_key(name: &str) -> String {
    let base = name.split('@').next().unwrap_or(name);
    let parts: Vec<&str> = base.split(':').collect();
    match parts.len() {
        0 | 1 | 2 => base.to_string(),
        3 => format!("{}:{}", parts[0], parts[1]),
        // group:artifact:version:classifier
        _ => format!("{}:{}:{}", parts[0], parts[1], parts[3]),
    }
}

/// 从 Maven 坐标里取 classifier（第 4 段），没有则 None
fn classifier_from_name(name: &str) -> Option<&str> {
    let base = name.split('@').next().unwrap_or(name);
    let parts: Vec<&str> = base.split(':').collect();
    if parts.len() >= 4 {
        Some(parts[3])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_drops_version() {
        assert_eq!(library_key("org.ow2.asm:asm:9.7.1"), "org.ow2.asm:asm");
    }

    #[test]
    fn key_keeps_classifier() {
        assert_eq!(
            library_key("net.minecraftforge:forge:1.20.1-47.4.22:universal"),
            "net.minecraftforge:forge:universal"
        );
    }
}
