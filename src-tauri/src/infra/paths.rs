use std::path::PathBuf;

const APP_DIR: &str = "mc-launcher";

/// 应用数据根目录
/// Windows: %APPDATA%\mc-launcher
pub fn data_dir() -> Option<PathBuf> {
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
