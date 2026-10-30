use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 实例级设置，存在 `<versions>/<id>/instance.json`
///
/// 所有字段都是 Option / 默认值：没设置就用全局配置，
/// 这样删掉文件也不影响启动。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstanceConfig {
    /// 显示名（默认等于目录名）
    pub name: String,
    /// 最大堆内存 MB（None = 用全局默认 4096）
    pub memory_mb: Option<u32>,
    /// 指定 Java 可执行文件（None = 自动挑满足要求的）
    pub java_path: Option<String>,
    /// 追加的 JVM 参数
    pub extra_jvm: Vec<String>,
    /// 窗口尺寸（None = 854x480）
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// 独立游戏目录（None = 用全局 minecraft 目录）
    pub game_dir: Option<String>,
    /// 上次启动时间（unix 秒）
    pub last_played: Option<i64>,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            memory_mb: None,
            java_path: None,
            extra_jvm: Vec::new(),
            width: None,
            height: None,
            game_dir: None,
            last_played: None,
        }
    }
}

/// 给前端的实例信息（配置 + 从版本定义里读出来的实际信息）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfo {
    /// 目录名，也是版本 id
    pub id: String,
    pub name: String,
    /// 实际加载的版本（可能就是 id）
    pub version: String,
    pub kind: Option<String>,
    pub main_class: String,
    pub memory_mb: Option<u32>,
    pub java_path: Option<String>,
    /// 版本定义要求的 Java 大版本
    pub java_major: Option<u32>,
    pub extra_jvm: Vec<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub last_played: Option<i64>,
    /// instance.json 的修改时间（排序用）
    pub created_at: i64,
    /// 实例目录占用（不含共享库，只有 jar + natives）
    pub size_bytes: u64,
    pub game_dir: String,
    /// 客户端 jar 在不在
    pub has_jar: bool,
    /// 版本定义里的库总数
    pub library_count: usize,
    /// 继承链
    pub chain: Vec<String>,
}

fn instance_dir(mc_dir: &Path, id: &str) -> PathBuf {
    mc_dir.join("versions").join(id)
}

fn config_path(mc_dir: &Path, id: &str) -> PathBuf {
    instance_dir(mc_dir, id).join("instance.json")
}

/// 读实例设置（文件不存在或解析失败 → 全默认）
pub fn load(mc_dir: &Path, id: &str) -> InstanceConfig {
    let p = config_path(mc_dir, id);
    let mut cfg: InstanceConfig = std::fs::read_to_string(&p)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();

    if cfg.name.trim().is_empty() {
        cfg.name = id.to_string();
    }
    cfg
}

/// 写实例设置
pub fn save(mc_dir: &Path, id: &str, cfg: &InstanceConfig) -> Result<(), anyhow::Error> {
    let p = config_path(mc_dir, id);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(cfg)?;
    // 先写临时文件再改名，避免写一半断电留下坏文件
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &p)?;
    Ok(())
}

/// 列出所有实例（扫 versions 目录，只认有 .json 的）
pub fn list(mc_dir: &Path) -> Vec<InstanceInfo> {
    let versions_dir = mc_dir.join("versions");
    let Ok(entries) = std::fs::read_dir(&versions_dir) else {
        return Vec::new();
    };

    let mut out = Vec::new();

    for e in entries.flatten() {
        if !e.path().is_dir() {
            continue;
        }
        let id = e.file_name().to_string_lossy().to_string();
        let json_path = e.path().join(format!("{id}.json"));
        if !json_path.is_file() {
            continue;
        }

        let Ok(text) = std::fs::read_to_string(&json_path) else {
            continue;
        };
        let Ok(vj) = serde_json::from_str::<crate::core::models::VersionJson>(&text) else {
            continue;
        };
        let Ok(rv) = crate::services::install::version_json::from_merged(&vj) else {
            continue;
        };

        let cfg = load(mc_dir, &id);

        let created_at = std::fs::metadata(&json_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        out.push(InstanceInfo {
            name: cfg.name.clone(),
            memory_mb: cfg.memory_mb,
            java_path: cfg.java_path.clone(),
            java_major: rv.java_major,
            extra_jvm: cfg.extra_jvm.clone(),
            width: cfg.width,
            height: cfg.height,
            last_played: cfg.last_played,
            created_at,
            size_bytes: dir_size(&e.path()),
            game_dir: cfg
                .game_dir
                .clone()
                .unwrap_or_else(|| mc_dir.display().to_string()),
            has_jar: e.path().join(format!("{id}.jar")).is_file(),
            library_count: rv.platform_libraries().len(),
            chain: rv.chain.clone(),
            kind: rv.kind.clone(),
            main_class: rv.main_class.clone(),
            version: rv.id.clone(),
            id,
        });
    }

    // 最近玩过的排前面
    out.sort_by(|a, b| {
        b.last_played
            .unwrap_or(0)
            .cmp(&a.last_played.unwrap_or(0))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    out
}

/// 记录一次启动时间
pub fn touch(mc_dir: &Path, id: &str) -> Result<(), anyhow::Error> {
    let mut cfg = load(mc_dir, id);
    cfg.last_played = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    );
    save(mc_dir, id, &cfg)
}

/// 删除实例目录（只删 versions/<id>，不动共享库）
pub fn remove(mc_dir: &Path, id: &str) -> Result<u64, anyhow::Error> {
    let dir = instance_dir(mc_dir, id);
    if !dir.is_dir() {
        return Err(anyhow::anyhow!("实例 {id} 不存在"));
    }
    // 防止 id 里带路径穿越
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(anyhow::anyhow!("非法的实例 id"));
    }
    let freed = dir_size(&dir);
    std::fs::remove_dir_all(&dir).map_err(|e| anyhow::anyhow!("删除失败（游戏可能正在运行）: {e}"))?;
    Ok(freed)
}

/// 复制一个实例目录（用于「另存为副本」）
pub fn duplicate(mc_dir: &Path, from: &str, to: &str) -> Result<(), anyhow::Error> {
    let src = instance_dir(mc_dir, from);
    let dst = instance_dir(mc_dir, to);
    if !src.is_dir() {
        return Err(anyhow::anyhow!("源实例不存在"));
    }
    if dst.exists() {
        return Err(anyhow::anyhow!("{to} 已存在"));
    }

    copy_dir(&src, &dst)?;

    // 目标目录里的 json / jar 要改成新名字
    let old_json = dst.join(format!("{from}.json"));
    if old_json.is_file() {
        let text = std::fs::read_to_string(&old_json)?;
        // 把 id 字段改成新名字，其余保持
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&text) {
            v["id"] = serde_json::Value::String(to.to_string());
            std::fs::write(dst.join(format!("{to}.json")), serde_json::to_string_pretty(&v)?)?;
        }
        let _ = std::fs::remove_file(&old_json);
    }
    let old_jar = dst.join(format!("{from}.jar"));
    if old_jar.is_file() {
        let _ = std::fs::rename(&old_jar, dst.join(format!("{to}.jar")));
    }
    // instance.json 里的名字重置
    let mut cfg = load(mc_dir, to);
    cfg.name = to.to_string();
    save(mc_dir, to, &cfg)?;

    Ok(())
}

/// 目录总大小
pub fn dir_size(dir: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];

    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                stack.push(e.path());
            } else {
                total += meta.len();
            }
        }
    }

    total
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), anyhow::Error> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)?.flatten() {
        let from = e.path();
        let to = dst.join(e.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip() {
        let dir = std::env::temp_dir().join("mcl_inst_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("versions").join("1.21.11")).unwrap();

        let mut cfg = InstanceConfig::default();
        cfg.name = "我的世界".to_string();
        cfg.memory_mb = Some(8192);
        cfg.extra_jvm = vec!["-XX:+UseG1GC".to_string()];

        save(&dir, "1.21.11", &cfg).unwrap();
        let back = load(&dir, "1.21.11");
        assert_eq!(back.name, "我的世界");
        assert_eq!(back.memory_mb, Some(8192));
        assert_eq!(back.extra_jvm.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_name_falls_back_to_id() {
        let dir = std::env::temp_dir().join("mcl_inst_test2");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = load(&dir, "1.20.1");
        assert_eq!(cfg.name, "1.20.1");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
