//! 启动参数预览（不真正启动游戏）
//!
//! 用法：cargo run --example launch_preview <版本号> [玩家名]
//!
//! 这个工具读磁盘上已经装好的版本定义，走和正式启动完全相同的代码路径
//! 生成命令，然后只打印出来 —— 用来核对参数是不是拼对了。

use std::path::PathBuf;

use mc_launcher_lib::core::models::VersionJson;
use mc_launcher_lib::services::install::version_json;
use mc_launcher_lib::services::launch::{account, args, classpath, LaunchOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args_in: Vec<String> = std::env::args().collect();
    let version = args_in.get(1).map(|s| s.as_str()).unwrap_or("1.21.11");
    let player = args_in.get(2).map(|s| s.as_str()).unwrap_or("MyisaCN");

    let mc_dir = std::env::var("MC_LAUNCHER_DATA_DIR")
        .map(|d| PathBuf::from(d).join("minecraft"))
        .unwrap_or_else(|_| PathBuf::from("minecraft"));

    let json_path = mc_dir.join("versions").join(version).join(format!("{version}.json"));
    println!("── 版本定义 ──");
    println!("  路径: {}", json_path.display());
    if !json_path.is_file() {
        return Err(format!("文件不存在: {}", json_path.display()).into());
    }

    let text = std::fs::read_to_string(&json_path)?;
    let vj: VersionJson = serde_json::from_str(&text)?;
    let rv = version_json::from_merged(&vj)?;

    println!("  mainClass : {}", rv.main_class);
    println!("  库总数    : {}", rv.libraries.len());
    println!("  本平台库  : {}", rv.platform_libraries().len());
    println!("  natives   : {}", rv.native_classifiers().len());
    println!("  Java 需求 : {:?}", rv.java_major);
    println!("  assets    : {}", rv.asset_index.id);

    /* classpath */
    let cp_paths = classpath::build(&mc_dir, &rv);
    let missing = classpath::missing_files(&mc_dir, &rv);
    println!("\n── classpath ──");
    println!("  条目数: {}", cp_paths.len());
    println!("  缺失  : {}", if missing.is_empty() { "无 ✓".to_string() } else { format!("{} 个", missing.len()) });
    for m in missing.iter().take(5) {
        println!("    - {m}");
    }

    /* 参数 */
    let acct = account::Account::offline(player);
    let opt = LaunchOptions {
        java: PathBuf::from(r"C:\Program Files\Java\jdk-21.0.12\bin\java.exe"),
        mc_dir: mc_dir.clone(),
        natives_dir: mc_dir.join("versions").join(version).join("natives"),
        classpath: classpath::join(&cp_paths),
        account: acct.clone(),
        memory_mb: 4096,
        width: 854,
        height: 480,
        extra_jvm: Vec::new(),
    };

    println!("\n── 账户 ──");
    println!("  名字: {}", acct.name);
    println!("  UUID: {}", acct.uuid_dashed());

    let jvm = args::build_jvm_args(&rv, &opt);
    println!("\n── JVM 参数（{} 项）──", jvm.len());
    for a in &jvm {
        let shown = if a.len() > 150 { format!("{}…<共{}字符>", &a[..120], a.len()) } else { a.clone() };
        println!("  {shown}");
    }

    let game = args::build_game_args(&rv, &opt);
    println!("\n── 游戏参数（{} 项）──", game.len());
    for a in &game {
        println!("  {a}");
    }

    println!("\n── 完整命令预览 ──");
    println!("{}", args::preview_command(&rv, &opt));

    /* 关键参数自检 */
    println!("\n── 自检 ──");
    let checks = [
        ("有 -Xmx", jvm.iter().any(|a| a.starts_with("-Xmx"))),
        ("有 -cp", jvm.iter().any(|a| a == "-cp")),
        ("有 natives 路径", jvm.iter().any(|a| a.contains("java.library.path"))),
        ("有 --username", game.iter().any(|a| a == "--username")),
        ("有 --version", game.iter().any(|a| a == "--version")),
        ("有 --gameDir", game.iter().any(|a| a == "--gameDir")),
        ("有 --assetIndex", game.iter().any(|a| a == "--assetIndex")),
        ("有 --uuid", game.iter().any(|a| a == "--uuid")),
        ("无残留 ${", !jvm.iter().chain(game.iter()).any(|a| a.contains("${"))),
    ];
    let mut all_ok = true;
    for (name, ok) in checks {
        println!("  {} {name}", if ok { "✓" } else { "✗" });
        if !ok {
            all_ok = false;
        }
    }

    if all_ok {
        println!("\n全部自检通过 ✓");
    } else {
        println!("\n有自检项失败 ✗");
    }

    Ok(())
}
