use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::State;

use crate::infra::downloader::{FileDownloader, FileTask, Outcome};
use crate::infra::task::TaskHandle;
use crate::services::install::version_json;
use crate::services::version;
use crate::state::AppState;

/// 安装（M1：解析版本 + 落地客户端 jar）的结果报告
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    pub id: String,
    pub main_class: String,
    pub library_count: usize,
    pub platform_library_count: usize,
    pub native_count: usize,
    pub asset_index_id: String,
    pub client_size: u64,
    pub client_skipped: bool,
    pub chain: Vec<String>,
    pub json_path: String,
    pub game_dir: String,
    pub elapsed_ms: u64,
}

/// 安装一个版本（M1 范围：版本定义 + 客户端 jar）
#[tauri::command]
pub async fn install_version(
    state: State<'_, AppState>,
    id: String,
) -> Result<InstallReport, String> {
    let task = state.tasks.create(format!("安装 {id}"));
    let started = std::time::Instant::now();

    match do_install(&state, &task, &id, started).await {
        Ok(report) => {
            task.finish(format!(
                "{} 已就绪 · {} 个库 · jar {:.1} MB",
                report.id,
                report.platform_library_count,
                report.client_size as f64 / 1024.0 / 1024.0
            ))
            .await;
            Ok(report)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("任务已取消") {
                task.cancelled().await;
            } else {
                task.fail(msg.clone()).await;
            }
            Err(msg)
        }
    }
}

async fn do_install(
    state: &AppState,
    task: &TaskHandle,
    id: &str,
    started: std::time::Instant,
) -> Result<InstallReport, anyhow::Error> {
    let mc_dir = version::default_mc_dir();
    let versions_dir = mc_dir.join("versions");
    std::fs::create_dir_all(&versions_dir)?;

    let report = |rv: &version_json::ResolvedVersion,
                  client_size: u64,
                  client_skipped: bool,
                  json_path: std::path::PathBuf|
     -> InstallReport {
        let platform_libs = rv.platform_libraries();
        InstallReport {
            id: rv.id.clone(),
            main_class: rv.main_class.clone(),
            library_count: rv.libraries.len(),
            platform_library_count: platform_libs.len(),
            native_count: rv.native_classifiers().len(),
            asset_index_id: rv.asset_index.id.clone(),
            client_size,
            client_skipped,
            chain: rv.chain.clone(),
            json_path: json_path.display().to_string(),
            game_dir: mc_dir.display().to_string(),
            elapsed_ms: started.elapsed().as_millis() as u64,
        }
    };

    // ① 版本清单（有缓存，通常秒开）
    task.progress(0.02, "读取版本清单").await;
    let manifest = version::fetch_manifest(&state.downloader, false).await?;

    // ② 解析 + inheritsFrom 合并
    let rv = version_json::resolve(&state.downloader, &versions_dir, &manifest, id, task).await?;

    // ③ 下载客户端 jar（单文件下载器的第一次真实使用）
    task.progress(0.35, "下载客户端 jar").await;

    let jar_path = versions_dir.join(id).join(format!("{id}.jar"));
    let file_task = FileTask::new(rv.client.url.clone(), jar_path.clone())
        .with_sha1(rv.client.sha1.clone())
        .with_size(rv.client.size);

    let expected = rv.client.size.unwrap_or(0);
    let last_push = AtomicU64::new(0);

    let outcome = FileDownloader::new()
        .download(&file_task, |done, total| {
            // 节流：最多每 120ms 推一次，避免刷爆前端事件
            let now = now_ms();
            if now.saturating_sub(last_push.load(Ordering::Relaxed)) < 120 {
                return;
            }
            last_push.store(now, Ordering::Relaxed);

            let total = total.unwrap_or(expected).max(1);
            let ratio = (done as f64 / total as f64).clamp(0.0, 1.0) as f32;
            task.progress_sync(
                0.35 + ratio * 0.60,
                format!(
                    "下载客户端 jar {:.1} / {:.1} MB",
                    done as f64 / 1024.0 / 1024.0,
                    total as f64 / 1024.0 / 1024.0
                ),
            );
        })
        .await?;

    let skipped = outcome == Outcome::Skipped;

    // ④ 收尾
    task.progress(0.97, "写入版本定义").await;
    let json_path = versions_dir.join(id).join(format!("{id}.json"));
    let client_size = std::fs::metadata(&jar_path).map(|m| m.len()).unwrap_or(0);

    let r = report(&rv, client_size, skipped, json_path);

    tracing::info!(
        "安装完成 {} | 继承链 {:?} | 库 {} (本平台 {}) | natives {} | assetIndex {} | mainClass {} | jar {} 字节{}",
        r.id,
        r.chain,
        r.library_count,
        r.platform_library_count,
        r.native_count,
        r.asset_index_id,
        r.main_class,
        r.client_size,
        if r.client_skipped { "（已存在，跳过）" } else { "" }
    );

    Ok(r)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
