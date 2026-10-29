use std::path::Path;
use std::time::Duration;

use sha1::{Digest, Sha1};

/// 一个待下载文件
#[derive(Debug, Clone)]
pub struct FileTask {
    pub url: String,
    pub dest: std::path::PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

impl FileTask {
    pub fn new(url: impl Into<String>, dest: impl Into<std::path::PathBuf>) -> Self {
        Self {
            url: url.into(),
            dest: dest.into(),
            sha1: None,
            size: None,
        }
    }

    pub fn with_sha1(mut self, sha1: Option<String>) -> Self {
        self.sha1 = sha1;
        self
    }

    pub fn with_size(mut self, size: Option<u64>) -> Self {
        self.size = size;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// 磁盘上已有且校验通过，跳过下载
    Skipped,
    /// 本次真正下载完成
    Downloaded,
}

/// 单文件下载器：SHA1 校验 + 进度回调 + 失败重试
///
/// 与 `http::Downloader` 的区别：
/// - `http::Downloader` 面向小型 JSON（读进内存）
/// - 这个面向大文件（jar / assets），流式落盘 + 边下边校验
pub struct FileDownloader {
    client: reqwest::Client,
}

impl Default for FileDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDownloader {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("mc-launcher/0.1.0")
            .timeout(Duration::from_secs(600))
            .connect_timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(32)
            .build()
            .expect("failed to build download client");
        Self { client }
    }

    /// 磁盘上已有文件是否可用（有 sha1 就必须匹配）
    pub fn verify_existing(task: &FileTask) -> bool {
        if !task.dest.is_file() {
            return false;
        }
        match task.sha1.as_deref() {
            None => true,
            Some(expected) => file_sha1(&task.dest)
                .map(|h| h.eq_ignore_ascii_case(expected))
                .unwrap_or(false),
        }
    }

    /// 下载一个文件。
    ///
    /// `on_progress(已下载字节, 总字节)` 会被高频调用，调用方自己做节流。
    pub async fn download<F>(
        &self,
        task: &FileTask,
        mut on_progress: F,
    ) -> Result<Outcome, anyhow::Error>
    where
        F: FnMut(u64, Option<u64>),
    {
        // 已存在且校验通过 → 跳过
        if Self::verify_existing(task) {
            let total = task.size.or_else(|| {
                std::fs::metadata(&task.dest).ok().map(|m| m.len())
            });
            on_progress(total.unwrap_or(0), total);
            return Ok(Outcome::Skipped);
        }

        if let Some(parent) = task.dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 先写 .part，校验通过后再 rename，避免半截文件被当成完整文件
        let tmp = task.dest.with_extension("part");
        let mut last_err: Option<anyhow::Error> = None;

        for attempt in 0..3u32 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(500u64 << attempt)).await;
            }

            match self.try_once(task, &tmp, &mut on_progress).await {
                Ok(()) => {
                    if let Some(expected) = task.sha1.as_deref() {
                        let actual = file_sha1(&tmp)?;
                        if !actual.eq_ignore_ascii_case(expected) {
                            let _ = std::fs::remove_file(&tmp);
                            last_err = Some(anyhow::anyhow!(
                                "SHA1 校验失败: 实际 {actual} 期望 {expected}"
                            ));
                            continue;
                        }
                    }
                    std::fs::rename(&tmp, &task.dest)?;
                    return Ok(Outcome::Downloaded);
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&tmp);
                    tracing::warn!("下载失败(第 {} 次) {}: {e}", attempt + 1, task.url);
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("下载失败: {}", task.url)))
    }

    async fn try_once<F>(
        &self,
        task: &FileTask,
        tmp: &Path,
        on_progress: &mut F,
    ) -> Result<(), anyhow::Error>
    where
        F: FnMut(u64, Option<u64>),
    {
        use tokio::io::AsyncWriteExt;

        let mut resp = self.client.get(&task.url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("HTTP {} ({})", resp.status(), task.url));
        }

        let total = resp.content_length().or(task.size);
        let mut file = tokio::fs::File::create(tmp).await?;
        let mut done: u64 = 0;

        while let Some(chunk) = resp.chunk().await? {
            file.write_all(&chunk).await?;
            done += chunk.len() as u64;
            on_progress(done, total);
        }

        file.flush().await?;
        drop(file);
        Ok(())
    }
}

/// 计算文件 SHA1（小写十六进制）
pub fn file_sha1(path: &Path) -> Result<String, anyhow::Error> {
    use std::io::Read;

    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];

    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}
