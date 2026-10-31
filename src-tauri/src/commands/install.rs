use serde::Serialize;
use tauri::State;

use crate::infra::task::TaskHandle;
use crate::services::install::{fabric, natives, plan, runner, version_json};
use crate::services::version;
use crate::state::AppState;

/// 安装结果报告（M2：版本定义 + 全部库 + 资源 + natives）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    pub id: String,
    pub main_class: String,
    pub library_count: usize,
    pub platform_library_count: usize,
    pub native_count: usize,
    pub native_extracted: usize,
    pub asset_count: usize,
    pub asset_index_id: String,
    pub client_size: u64,
    pub chain: Vec<String>,
    pub json_path: String,
    pub game_dir: String,
    /// 本次真正下载的文件数
    pub downloaded: usize,
    /// 已存在且校验通过、跳过的文件数
    pub skipped: usize,
    /// 失败的文件（相对路径 + 原因）
    pub failed: Vec<String>,
    /// 下载总字节
    pub total_bytes: u64,
    pub elapsed_ms: u64,
}

/// 安装一个版本（完整安装）
#[tauri::command]
pub async fn install_version(
    state: State<'_, AppState>,
    id: String,
) -> Result<InstallReport, String> {
    let task = state.tasks.create(format!("安装 {id}"));
    let started = std::time::Instant::now();

    match do_install(&state, &task, &id, started).await {
        Ok(report) => {
            let summary = if report.failed.is_empty() {
                format!(
                    "{} 安装完成 · {} 个文件 · {:.1} MB · {:.0} 秒",
                    report.id,
                    report.downloaded,
                    report.total_bytes as f64 / 1024.0 / 1024.0,
                    report.elapsed_ms as f64 / 1000.0
                )
            } else {
                format!(
                    "{} 基本完成 · {}/{} 成功 · {} 个失败",
                    report.id,
                    report.downloaded + report.skipped,
                    report.downloaded + report.skipped + report.failed.len(),
                    report.failed.len()
                )
            };
            task.finish(summary).await;
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

    // ① 版本清单
    task.progress(0.02, "读取版本清单").await;
    let manifest = version::fetch_manifest(&state.downloader, false).await?;

    // ② 解析 + inheritsFrom 合并
    let rv = version_json::resolve(&state.downloader, &versions_dir, &manifest, id, task).await?;

    // ③ 计算下载清单
    task.progress(0.08, "计算下载清单").await;
    let dl_plan = plan::build_plan(&state.downloader, &mc_dir, &rv, task).await?;

    // ④ 并发下载全部内容
    task.progress(0.10, "开始下载").await;
    let limit = state.config.read().await.download_limit_kbps;
    let stats = runner::run(&dl_plan, &mc_dir, 16, limit, task).await?;

    // ⑤ 解压 natives
    task.progress(0.96, "解压 natives").await;
    let native_dest = plan::natives_dir(&mc_dir, &rv.id);
    let native_jars: Vec<std::path::PathBuf> = dl_plan
        .natives
        .iter()
        .map(|n| n.dest.clone())
        .filter(|p| p.is_file())
        .collect();
    let exclude: Vec<String> = rv
        .libraries
        .iter()
        .filter_map(|l| l.extract.as_ref())
        .flat_map(|e| e.exclude.clone())
        .collect();
    let (_jars, native_files) = natives::extract_all(&native_jars, &native_dest, &exclude)?;

    // ⑥ 汇总
    let jar_path = versions_dir.join(id).join(format!("{id}.jar"));
    let client_size = std::fs::metadata(&jar_path).map(|m| m.len()).unwrap_or(0);
    let json_path = versions_dir.join(id).join(format!("{id}.json"));

    let report = InstallReport {
        id: rv.id.clone(),
        main_class: rv.main_class.clone(),
        library_count: rv.libraries.len(),
        platform_library_count: rv.platform_libraries().len(),
        native_count: rv.native_classifiers().len(),
        native_extracted: native_files,
        asset_count: dl_plan.assets.len(),
        asset_index_id: rv.asset_index.id.clone(),
        client_size,
        chain: rv.chain.clone(),
        json_path: json_path.display().to_string(),
        game_dir: mc_dir.display().to_string(),
        downloaded: stats.downloaded,
        skipped: stats.skipped,
        failed: stats.failed.clone(),
        total_bytes: stats.bytes,
        elapsed_ms: started.elapsed().as_millis() as u64,
    };

    tracing::info!(
        "安装完成 {} | 库 {}/{} | natives {} → {} 文件 | assets {} | 下载 {} 跳过 {} 失败 {} | {:.1} MB | {} ms",
        report.id,
        report.platform_library_count,
        report.library_count,
        report.native_count,
        report.native_extracted,
        report.asset_count,
        report.downloaded,
        report.skipped,
        report.failed.len(),
        report.total_bytes as f64 / 1024.0 / 1024.0,
        report.elapsed_ms
    );

    for f in report.failed.iter().take(5) {
        tracing::warn!("下载失败: {f}");
    }

    Ok(report)
}


/* ─────────────── Fabric 加载器 ─────────────── */

/// 列出某个游戏版本能装哪些 Fabric loader
#[tauri::command]
pub async fn list_fabric_loaders(
    state: State<'_, AppState>,
    game: String,
) -> Result<Vec<fabric::FabricLoader>, String> {
    fabric::list_loaders(&state.downloader, &game)
        .await
        .map_err(|e| e.to_string())
}

/// 安装 Fabric：先落版本定义，再走通用安装链路
///
/// `loader` 传 None 表示用最新的稳定版。
#[tauri::command]
pub async fn install_fabric(
    state: State<'_, AppState>,
    game: String,
    loader: Option<String>,
) -> Result<InstallReport, String> {
    let mc_dir = version::default_mc_dir();
    let versions_dir = mc_dir.join("versions");
    std::fs::create_dir_all(&versions_dir).map_err(|e| e.to_string())?;

    let task = state.tasks.create(format!("安装 Fabric · {game}"));
    let started = std::time::Instant::now();

    // ① 拿 profile 写进版本目录（这一步会挑 loader）
    let id = match fabric::prepare(&state.downloader, &versions_dir, &game, loader.as_deref()).await {
        Ok(id) => id,
        Err(e) => {
            let msg = e.to_string();
            task.fail(msg.clone()).await;
            return Err(msg);
        }
    };

    // ② 剩下的跟装原版完全一样
    match do_install(&state, &task, &id, started).await {
        Ok(report) => {
            let summary = if report.failed.is_empty() {
                format!(
                    "{} 安装完成 · {} 个文件 · {:.1} MB · {:.0} 秒",
                    report.id,
                    report.downloaded,
                    report.total_bytes as f64 / 1024.0 / 1024.0,
                    report.elapsed_ms as f64 / 1000.0
                )
            } else {
                format!(
                    "{} 基本完成 · {} 个失败",
                    report.id,
                    report.failed.len()
                )
            };
            task.finish(summary).await;
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
