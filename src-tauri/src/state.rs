use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::models::AppConfig;
use crate::infra::http::Downloader;
use crate::infra::task::TaskManager;

/// 全局应用状态 —— 通过 tauri::State 注入到命令里
pub struct AppState {
    pub app: tauri::AppHandle,
    pub tasks: Arc<TaskManager>,
    pub downloader: Arc<Downloader>,
    pub config: RwLock<AppConfig>,
}

impl AppState {
    pub fn new(app: tauri::AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let config = AppConfig::load().unwrap_or_default();
        let downloader = Downloader::new(config.download_source.clone());

        Ok(Self {
            tasks: Arc::new(TaskManager::new(app.clone())),
            downloader: Arc::new(downloader),
            config: RwLock::new(config),
            app,
        })
    }
}
