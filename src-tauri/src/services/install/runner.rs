use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::infra::downloader::{FileDownloader, FileTask, Outcome};
use crate::infra::http::MIRROR_BASE;
use crate::infra::task::TaskHandle;

use super::plan::{asset_path, DownloadPlan, PlannedFile};

const ASSET_OFFICIAL: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Default)]
pub struct RunStats {
    pub total: usize,
    pub downloaded: usize,
    pub skipped: usize,
    pub failed: Vec<String>,
    pub bytes: u64,
    pub elapsed_ms: u64,
}

/// 并发跑完整个下载清单
pub async fn run(
    plan: &DownloadPlan,
    mc_dir: &std::path::Path,
    concurrency: usize,
    task: &TaskHandle,
) -> Result<RunStats, anyhow::Error> {
    let started = std::time::Instant::now();
    let files = collect_files(plan, mc_dir);
    let total = files.len();
    let total_bytes = files.iter().filter_map(|f| f.size).sum::<u64>();

    tracing::info!(
        "开始下载 {total} 个文件，合计 {:.1} MB，并发 {concurrency}",
        total_bytes as f64 / 1024.0 / 1024.0
    );

    let done = Arc::new(AtomicUsize::new(0));
    let skipped = Arc::new(AtomicUsize::new(0));
    let bytes = Arc::new(AtomicU64::new(0));
    let failed = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let last_push = Arc::new(AtomicU64::new(0));

    let dl = Arc::new(FileDownloader::new());
    let sem = Arc::new(tokio::sync::Semaphore::new(concurrency.max(1)));
    let mut set = tokio::task::JoinSet::new();

    for file in files {
        let permit = sem.clone().acquire_owned().await?;
        let dl = dl.clone();
        let done = done.clone();
        let skipped = skipped.clone();
        let bytes = bytes.clone();
        let failed = failed.clone();
        let last_push = last_push.clone();
        let task = task.clone();
        let total = total;

        set.spawn(async move {
            let _permit = permit;

            let task_file = FileTask::new(file.url.clone(), file.dest.clone())
                .with_sha1(file.sha1.clone())
                .with_size(file.size);

            let mut last_bytes = 0u64;
            let mut result = dl
                .download(&task_file, |b, _| {
                    last_bytes = b;
                })
                .await;

            // 主源失败 → 换镜像再来
            if result.is_err() {
                if let Some(mirror) = &file.mirror_url {
                    tracing::warn!("主源失败，回退镜像: {}", file.relative);
                    let mirror_task = FileTask::new(mirror.clone(), file.dest.clone())
                        .with_sha1(file.sha1.clone())
                        .with_size(file.size);
                    result = dl
                        .download(&mirror_task, |b, _| {
                            last_bytes = b;
                        })
                        .await;
                }
            }

            match result {
                Ok(outcome) => {
                    if outcome == Outcome::Skipped {
                        skipped.fetch_add(1, Ordering::Relaxed);
                    }
                    bytes.fetch_add(last_bytes, Ordering::Relaxed);
                }
                Err(e) => {
                    failed
                        .lock()
                        .await
                        .push(format!("{}: {e}", file.relative));
                }
            }

            let finished = done.fetch_add(1, Ordering::Relaxed) + 1;

            // 进度节流：200ms
            let now = now_ms();
            if now.saturating_sub(last_push.load(Ordering::Relaxed)) >= 200 || finished == total {
                last_push.store(now, Ordering::Relaxed);
                let got = bytes.load(Ordering::Relaxed);
                let ratio = finished as f32 / total.max(1) as f32;
                task.progress_sync(
                    0.10 + ratio * 0.85,
                    format!(
                        "{finished}/{total} 个文件 · {:.1} MB",
                        got as f64 / 1024.0 / 1024.0
                    ),
                );
            }
        });
    }

    while let Some(joined) = set.join_next().await {
        if let Err(e) = joined {
            tracing::error!("下载任务崩溃: {e}");
        }
    }

    let failed_list = failed.lock().await.clone();
    let stats = RunStats {
        total,
        downloaded: done.load(Ordering::Relaxed) - skipped.load(Ordering::Relaxed)
            - failed_list.len(),
        skipped: skipped.load(Ordering::Relaxed),
        failed: failed_list,
        bytes: bytes.load(Ordering::Relaxed),
        elapsed_ms: started.elapsed().as_millis() as u64,
    };

    if !stats.failed.is_empty() {
        tracing::warn!("有 {} 个文件下载失败", stats.failed.len());
    }

    Ok(stats)
}

/// 把 plan 展开成统一的文件列表
fn collect_files(plan: &DownloadPlan, mc_dir: &std::path::Path) -> Vec<PlannedFile> {
    let mut files = Vec::with_capacity(
        plan.libraries.len() + plan.natives.len() + plan.assets.len() + 2,
    );

    files.extend(plan.libraries.iter().cloned());
    files.extend(plan.natives.iter().cloned());

    if let Some(idx) = &plan.assets_index {
        files.push(idx.clone());
    }

    let assets_dir = mc_dir.join("assets");
    for a in &plan.assets {
        let prefix = &a.hash[..2.min(a.hash.len())];
        files.push(PlannedFile {
            // 资源对象走 BMCLAPI（国内快），失败回退官方
            url: format!("{MIRROR_BASE}/assets/{prefix}/{}", a.hash),
            mirror_url: Some(format!("{ASSET_OFFICIAL}/{prefix}/{}", a.hash)),
            dest: asset_path(&assets_dir, &a.hash),
            relative: format!("assets/{prefix}/{}", a.hash),
            sha1: Some(a.hash.clone()),
            size: Some(a.size),
            name: format!("asset {}", &a.hash[..8.min(a.hash.len())]),
        });
    }

    files
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
