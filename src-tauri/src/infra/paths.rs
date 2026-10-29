use std::path::PathBuf;

const APP_DIR: &str = "mc-launcher";

/// 应用数据根目录
///
/// 优先级：
/// 1. 环境变量 `MC_LAUNCHER_DATA_DIR`（开发期把数据留在项目盘，不占 C 盘）
/// 2. Windows: `%APPDATA%\mc-launcher`（用户装好之后走这个）
pub fn data_dir() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("MC_LAUNCHER_DATA_DIR") {
        if !custom.trim().is_empty() {
            return Some(PathBuf::from(custom));
        }
    }
    dirs::data_dir().map(|d| d.join(APP_DIR))
}

/// 配置目录（同 data_dir）
pub fn config_dir() -> Option<PathBuf> {
    data_dir()
}

/// 缓存目录
pub fn cache_dir() -> Option<PathBuf> {
    data_dir().map(|d| d.join("cache"))
}

/// 日志目录
pub fn log_dir() -> Option<PathBuf> {
    data_dir().map(|d| d.join("logs"))
}

/// 确保目录存在并返回
pub fn ensure_dir(p: PathBuf) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(&p)?;
    Ok(p)
}
