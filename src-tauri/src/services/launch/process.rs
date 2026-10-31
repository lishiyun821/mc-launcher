use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// 启动后的进程信息
pub struct GameProcess {
    pub child: Child,
    pub pid: u32,
    pub command_line: String,
}

/// 启动 Java 进程
///
/// - 工作目录设为游戏目录（存档/日志才会落在正确位置）
/// - stdout/stderr 都接管道，由上层转成事件推给前端
#[allow(clippy::too_many_arguments)]
pub async fn spawn(
    java: &Path,
    jvm_args: &[String],
    main_class: &str,
    game_args: &[String],
    work_dir: &Path,
    log_file: Option<PathBuf>,
) -> Result<GameProcess, anyhow::Error> {
    if !java.is_file() {
        return Err(anyhow::anyhow!("找不到 Java: {}", java.display()));
    }
    std::fs::create_dir_all(work_dir)?;

    // Windows 上必须用 javaw.exe 启动。
    //
    // java.exe 是控制台程序，配合 CREATE_NO_WINDOW（不弹黑框）会失去控制台，
    // 结果 JVM 直接以退出码 1 结束，stdout/stderr 一个字都不给 —— 极难排查。
    // javaw.exe 是同一份 JVM 的窗口版，没有这个问题，也是 PCL 等启动器的做法。
    #[cfg(windows)]
    let java = {
        let javaw = java.with_file_name("javaw.exe");
        if javaw.is_file() {
            tracing::debug!("使用 javaw 启动: {}", javaw.display());
            javaw
        } else {
            java.to_path_buf()
        }
    };
    #[cfg(not(windows))]
    let java = java.to_path_buf();

    let mut cmd = Command::new(&java);
    cmd.current_dir(work_dir)
        .args(jvm_args)
        .arg(main_class)
        .args(game_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        // 让游戏窗口不要弹出一个控制台；CREATE_NO_WINDOW
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let command_line = format!(
        "\"{}\" {} {} {}",
        java.display(),
        jvm_args.join(" "),
        main_class,
        game_args.join(" ")
    );

    tracing::info!("启动游戏完整命令: {}", &command_line);

    // 把完整命令行落一份，出问题时能直接复制到手敲一遍复现
    if let Some(base) = &log_file {
        let dump = base.with_extension("cmd.txt");
        let _ = std::fs::write(&dump, &command_line);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("启动 Java 失败: {e}"))?;

    let pid = child.id().unwrap_or(0);
    tracing::info!(
        "进程已起 pid={pid} | jvm 参数 {} 个 | main={} | game 参数 {} 个",
        jvm_args.len(),
        main_class,
        game_args.len()
    );

    // 日志落盘 + 推前端
    let log_path = log_file;
    if let Some(out) = child.stdout.take() {
        spawn_pipe_reader(out, log_path.clone(), "stdout");
    }
    if let Some(err) = child.stderr.take() {
        spawn_pipe_reader(err, log_path, "stderr");
    }

    Ok(GameProcess {
        child,
        pid,
        command_line,
    })
}

/// 读一条管道，边写日志文件边输出到 tracing
fn spawn_pipe_reader<R>(reader: R, log_path: Option<PathBuf>, source: &'static str)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        let mut file = log_path.and_then(|p| {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .ok()
        });

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            if let Some(f) = file.as_mut() {
                use std::io::Write;
                let _ = writeln!(f, "[{source}] {line}");
            }
            // 游戏日志通常很啰嗦，用 debug 级别，同时保留到 tracing 日志文件
            if source == "stderr" {
                tracing::warn!("[游戏stderr] {line}");
            } else {
                tracing::info!("[游戏] {line}");
            }
        }
    });
}

/// 等进程结束，返回退出码
pub async fn wait(mut child: Child) -> Option<i32> {
    match child.wait().await {
        Ok(status) => status.code(),
        Err(e) => {
            tracing::warn!("等待游戏进程失败: {e}");
            None
        }
    }
}
