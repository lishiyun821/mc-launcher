use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// 初始化日志：控制台 + 文件（按天滚动）
/// 必须在应用启动最早期调用
pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,mc_launcher_lib=debug"));

    let registry = tracing_subscriber::registry().with(filter);

    if let Some(dir) = super::paths::log_dir() {
        if std::fs::create_dir_all(&dir).is_ok() {
            let file_appender = tracing_appender::rolling::daily(&dir, "launcher.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            // 全局泄漏 guard，保证进程生命周期内日志不丢
            Box::leak(Box::new(guard));

            registry
                .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
                .with(fmt::layer().with_ansi(true))
                .init();
            return;
        }
    }

    // 没有可写目录时就只输出到控制台
    registry.with(fmt::layer()).init();
}
