//! 续期链路验证：cargo run --example test_refresh
//!
//! 直接走 refresh_token → MS token → Minecraft token，验证这条链还通不通。

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("MC_LAUNCHER_DATA_DIR", r"E:\code\mc-launcher\data");

    use mc_launcher_lib::services::auth::{microsoft, store};

    let acc = match store::active() {
        Some(a) => a,
        None => {
            println!("没有活动账户");
            return Ok(());
        }
    };

    println!("账户: {} ({})", acc.name, acc.kind);
    println!("当前 expires_at: {}", acc.expires_at);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    println!("现在:            {}", now);
    println!("剩余:            {} 秒 ({} 小时)", acc.expires_at - now, (acc.expires_at - now) / 3600);
    println!("已过期吗:        {}", acc.is_expired());
    println!("有 refresh_token: {}", !acc.refresh_token.is_empty());

    println!("\n开始续期…");
    let http = reqwest::Client::builder()
        .user_agent("mc-launcher/0.1.0")
        .build()?;

    match microsoft::refresh_ms_token(&http, &acc.refresh_token).await {
        Ok(ms) => {
            println!("① MS 令牌刷新成功");
            match microsoft::login_with_ms_token(&http, ms).await {
                Ok(outcome) => {
                    println!("② Xbox 链路成功，xuid = {}", outcome.xuid);
                    println!("③ Minecraft 令牌拿到，expires_in = {} 秒", outcome.minecraft_token.expires_in);
                    println!("④ 玩家: {} / {}", outcome.profile.name, outcome.profile.id);
                    println!("\n✓ 整条续期链路正常");
                }
                Err(e) => println!("✗ Xbox 链路失败: {e}"),
            }
        }
        Err(e) => println!("✗ MS 刷新失败（refresh_token 可能已失效）: {e}"),
    }

    Ok(())
}
