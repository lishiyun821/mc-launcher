use std::path::PathBuf;

use crate::core::models::{RawManifest, VersionManifest};
use crate::infra::http::{Downloader, MANIFEST_PATH};
use crate::infra::paths;

/// 版本清单缓存有效期：1 小时
const MANIFEST_TTL_SECS: u64 = 3600;

/// 拉取版本清单（带缓存 + 镜像回退）
pub async fn fetch_manifest(
    dl: &Downloader,
    force_refresh: bool,
) -> Result<VersionManifest, anyhow::Error> {
    let cache_file = paths::cache_dir()
        .map(|d| d.join("version_manifest_v2.json"))
        .ok_or_else(|| anyhow::anyhow!("无法确定缓存目录"))?;

    let raw: RawManifest = dl
        .get_json_cached(MANIFEST_PATH, &cache_file, force_refresh, MANIFEST_TTL_SECS)
        .await?;

    tracing::info!("版本清单加载完成，共 {} 个版本", raw.versions.len());
    Ok(raw.into())
}

/// 默认的 .minecraft 目录
pub fn default_mc_dir() -> PathBuf {
    dirs::data_dir()
        .map(|d| d.join(".minecraft"))
        .unwrap_or_else(|| PathBuf::from(".minecraft"))
}

/// 扫描本地已安装版本（versions 目录下的每个子目录）
pub fn list_local_versions() -> Vec<String> {
    let versions_dir = default_mc_dir().join("versions");
    let mut out = Vec::new();

    let Ok(entries) = std::fs::read_dir(&versions_dir) else {
        return out;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // 只有含 <name>.json 的才算有效版本
        if path.join(format!("{name}.json")).exists() {
            out.push(name.to_string());
        }
    }

    out.sort();
    out
}
