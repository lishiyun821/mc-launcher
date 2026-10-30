use serde::Serialize;
use tauri::State;

use crate::core::models::JavaInfo;
use crate::services;
use crate::state::AppState;

#[tauri::command]
pub async fn scan_java() -> Result<Vec<JavaInfo>, String> {
    Ok(services::java::scan().await)
}

/* ─────────────── Java 运行时自动下载 ─────────────── */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub major: u32,
    pub path: String,
}

/// 列出启动器自己装好的 Java 运行时
#[tauri::command]
pub async fn list_java_runtimes() -> Result<Vec<RuntimeInfo>, String> {
    Ok(services::java_download::list_installed()
        .into_iter()
        .map(|(major, exe)| RuntimeInfo {
            major,
            path: exe.display().to_string(),
        })
        .collect())
}

/// 查某个大版本的最新 JRE 信息（不下载，给界面显示用）
#[tauri::command]
pub async fn java_remote_info(
    state: State<'_, AppState>,
    major: u32,
) -> Result<services::java_download::JreInfo, String> {
    services::java_download::fetch_latest(state.downloader.http(), major)
        .await
        .map_err(|e| e.to_string())
}

/// 下载并安装某个大版本的 JRE，返回 java 可执行文件路径
#[tauri::command]
pub async fn install_java(state: State<'_, AppState>, major: u32) -> Result<String, String> {
    let task = state.tasks.create(format!("安装 Java {major}"));
    let handle = task.clone();
    let http = state.downloader.http().clone();

    let result = services::java_download::install(&http, major, move |pct, detail| {
        handle.progress_sync(pct as f32 / 100.0, detail);
    })
    .await;

    match result {
        Ok(exe) => {
            let path = exe.display().to_string();
            task.finish(format!("Java {major} 已就绪")).await;
            tracing::info!("Java {major} 安装完成: {path}");
            Ok(path)
        }
        Err(e) => {
            let msg = e.to_string();
            task.fail(msg.clone()).await;
            Err(msg)
        }
    }
}
