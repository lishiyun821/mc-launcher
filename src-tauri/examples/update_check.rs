//! 更新检查验证：cargo run --example update_check

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http = reqwest::Client::builder()
        .user_agent("mc-launcher/0.1.0")
        .build()?;

    let info = mc_launcher_lib::commands::update::check(&http).await;
    println!("当前版本: {}", info.current);
    println!("最新版本: {:?}", info.latest);
    println!("有更新:   {}", info.has_update);
    println!("链接:     {}", info.url);
    println!("发布时间: {:?}", info.published_at);
    println!("错误:     {:?}", info.error);
    if let Some(n) = &info.notes {
        println!("说明前 200 字: {}", n.chars().take(200).collect::<String>());
    }
    Ok(())
}
