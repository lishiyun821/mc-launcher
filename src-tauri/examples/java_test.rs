//! Java 下载测试：cargo run --example java_test [major]

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let major: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(21);

    std::env::set_var("MC_LAUNCHER_DATA_DIR", r"E:\code\mc-launcher\data");

    let http = reqwest::Client::builder()
        .user_agent("mc-launcher/0.1.0")
        .build()?;

    let result = mc_launcher_lib::services::java_download::install(&http, major, |pct, msg| {
        println!("[{pct:3}%] {msg}");
    })
    .await;

    match result {
        Ok(p) => {
            println!("\n✓ java.exe = {}", p.display());
            // 跑一下看版本
            let out = std::process::Command::new(&p).arg("-version").output()?;
            let text = String::from_utf8_lossy(&out.stderr);
            for line in text.lines().take(2) {
                println!("  {line}");
            }
        }
        Err(e) => println!("\n✗ 失败: {e}"),
    }

    Ok(())
}
