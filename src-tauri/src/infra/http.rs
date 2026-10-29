use std::path::Path;
use std::time::Duration;

use serde::de::DeserializeOwned;
use tokio::time::sleep;

use crate::core::models::DownloadSource;

/// 官方源
pub const OFFICIAL_BASE: &str = "https://piston-meta.mojang.com";
/// BMCLAPI 国内镜像（公益，bangbang93 提供）
pub const MIRROR_BASE: &str = "https://bmclapi2.bangbang93.com";

pub const MANIFEST_PATH: &str = "/mc/game/version_manifest_v2.json";

/// 带镜像回退 + 重试的下载器
pub struct Downloader {
    client: reqwest::Client,
    prefer: DownloadSource,
}

impl Downloader {
    pub fn new(prefer: DownloadSource) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("mc-launcher/0.1.0 (+https://github.com/)")
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(16)
            .build()
            .expect("failed to build http client");

        Self { client, prefer }
    }

    /// 按偏好顺序返回候选 URL（首选 + 兜底）
    fn urls_for(&self, path: &str) -> Vec<String> {
        let official = format!("{OFFICIAL_BASE}{path}");
        let mirror = format!("{MIRROR_BASE}{path}");
        match self.prefer {
            DownloadSource::Official => vec![official, mirror],
            DownloadSource::Mirror => vec![mirror, official],
        }
    }

    /// 直接下载绝对 URL，带重试
    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, anyhow::Error> {
        let mut last_err: Option<anyhow::Error> = None;

        for attempt in 0..3u32 {
            if attempt > 0 {
                sleep(Duration::from_millis(400u64 << attempt)).await;
            }
            match self.client.get(url).send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        last_err = Some(anyhow::anyhow!("HTTP {} for {}", resp.status(), url));
                        continue;
                    }
                    match resp.bytes().await {
                        Ok(b) => return Ok(b.to_vec()),
                        Err(e) => last_err = Some(e.into()),
                    }
                }
                Err(e) => last_err = Some(e.into()),
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("下载失败: {url}")))
    }

    /// 按「相对路径」下载，自动在官方源和镜像之间回退
    pub async fn get_bytes_path(&self, path: &str) -> Result<Vec<u8>, anyhow::Error> {
        let mut last_err: Option<anyhow::Error> = None;

        for url in self.urls_for(path) {
            match self.get_bytes(&url).await {
                Ok(b) => {
                    tracing::debug!("命中源: {url}");
                    return Ok(b);
                }
                Err(e) => {
                    tracing::warn!("源失败 {url}: {e}");
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("所有源都失败: {path}")))
    }

    /// 取 JSON，同样带回退
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, anyhow::Error> {
        let bytes = self.get_bytes_path(path).await?;
        serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!("JSON 解析失败: {e}"))
    }

    /// 带磁盘缓存的 JSON 读取
    pub async fn get_json_cached<T: DeserializeOwned>(
        &self,
        path: &str,
        cache_file: &Path,
        force_refresh: bool,
        max_age_secs: u64,
    ) -> Result<T, anyhow::Error> {
        if !force_refresh && cache_file.exists() {
            if let Ok(meta) = std::fs::metadata(cache_file) {
                let fresh = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .map(|d| d.as_secs() < max_age_secs)
                    .unwrap_or(false);
                if fresh {
                    if let Ok(text) = std::fs::read(cache_file) {
                        if let Ok(v) = serde_json::from_slice::<T>(&text) {
                            tracing::debug!("命中缓存: {}", cache_file.display());
                            return Ok(v);
                        }
                    }
                }
            }
        }

        let bytes = self.get_bytes_path(path).await?;
        if let Some(parent) = cache_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(cache_file, &bytes);
        serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!("JSON 解析失败: {e}"))
    }
}
