//! 真实启动测试（会弹出游戏窗口）
//!
//! 用法：cargo run --example launch_run <版本号> [玩家名] [秒数]
//!
//! 走和正式启动完全相同的路径：拼 classpath → 生成参数 → spawn java。
//! 启动后会等 N 秒（默认 45），把游戏日志里的关键行打出来，然后结束。

use std::path::PathBuf;
use std::process::Stdio;

use mc_launcher_lib::core::models::VersionJson;
use mc_launcher_lib::services::install::version_json;
use mc_launcher_lib::services::launch::{account, args, classpath, LaunchOptions};
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argv: Vec<String> = std::env::args().collect();
    let version = argv.get(1).map(|s| s.as_str()).unwrap_or("1.21.11");
    let player = argv.get(2).map(|s| s.as_str()).unwrap_or("MyisaCN");
    let wait_secs: u64 = argv.get(3).and_then(|s| s.parse().ok()).unwrap_or(45);

    let mc_dir = std::env::var("MC_LAUNCHER_DATA_DIR")
        .map(|d| PathBuf::from(d).join("minecraft"))
        .unwrap_or_else(|_| PathBuf::from("minecraft"));

    let json_path = mc_dir.join("versions").join(version).join(format!("{version}.json"));
    let text = std::fs::read_to_string(&json_path)?;
    let vj: VersionJson = serde_json::from_str(&text)?;
    let rv = version_json::from_merged(&vj)?;

    let cp_paths = classpath::build(&mc_dir, &rv);
    let missing = classpath::missing_files(&mc_dir, &rv);
    if !missing.is_empty() {
        return Err(format!("缺 {} 个库文件，先装好再启动", missing.len()).into());
    }

    let java = argv
        .iter()
        .find(|a| a.ends_with("java.exe"))
        .cloned()
        .unwrap_or_else(|| r"C:\Program Files\Java\jdk-21.0.12\bin\java.exe".to_string());

    let acct = account::Account::offline(player);
    let opt = LaunchOptions {
        java: PathBuf::from(&java),
        mc_dir: mc_dir.clone(),
        natives_dir: mc_dir.join("versions").join(version).join("natives"),
        classpath: classpath::join(&cp_paths),
        account: acct.clone(),
        memory_mb: 4096,
        width: 854,
        height: 480,
        extra_jvm: Vec::new(),
    };

    let jvm = args::build_jvm_args(&rv, &opt);
    let game = args::build_game_args(&rv, &opt);

    println!("启动 {} | Java {} | {} 项 classpath | 玩家 {}", version, java, cp_paths.len(), player);
    println!("工作目录: {}", mc_dir.display());

    let mut cmd = tokio::process::Command::new(&java);
    cmd.current_dir(&mc_dir)
        .args(&jvm)
        .arg(&rv.main_class)
        .args(&game)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let pid = child.id().unwrap_or(0);
    println!("PID = {pid}\n");

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let out_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        let mut count = 0;
        while let Ok(Some(l)) = lines.next_line().await {
            count += 1;
            // 只打关键行，避免刷屏
            if l.contains("Setting user")
                || l.contains("Backend library")
                || l.contains("OpenAL")
                || l.contains("LWJGL")
                || l.contains("Narrator")
                || l.contains("OpenGL")
                || l.contains("Created:")
                || l.contains("Reloading")
                || count <= 12
            {
                println!("[out] {l}");
            }
        }
        count
    });

    let err_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        let mut v = Vec::new();
        while let Ok(Some(l)) = lines.next_line().await {
            v.push(l);
            if v.len() > 60 {
                v.remove(0);
            }
        }
        v
    });

    // 等到超时或者游戏退出
    let deadline = tokio::time::sleep(std::time::Duration::from_secs(wait_secs));
    tokio::pin!(deadline);

    tokio::select! {
        _ = &mut deadline => {
            println!("\n── {wait_secs} 秒到，结束进程 ──");
            let _ = child.kill().await;
        }
        status = child.wait() => {
            println!("\n── 游戏进程已退出: {status:?} ──");
        }
    }

    let out_count = out_task.await.unwrap_or(0);
    let err_lines = err_task.await.unwrap_or_default();

    println!("\n标准输出行数: {out_count}");
    if !err_lines.is_empty() {
        println!("标准错误（最后 10 行）:");
        for l in err_lines.iter().rev().take(10).rev() {
            println!("  [err] {l}");
        }
    }

    Ok(())
}
