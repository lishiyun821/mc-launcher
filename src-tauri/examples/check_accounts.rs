//! 账户数据检查：cargo run --example check_accounts

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("MC_LAUNCHER_DATA_DIR", r"E:\code\mc-launcher\data");

    let file = mc_launcher_lib::services::auth::store::load();
    println!("=== store ===");
    println!("active = {:?}", file.active);
    for a in &file.accounts {
        println!(
            "id={} name={} kind={} uuid={}",
            a.id, a.name, a.kind, a.uuid
        );
    }

    // 模拟 list_accounts 的 to_info + AccountsView 序列化
    let view = serde_json::json!({
        "accounts": file.accounts.iter().map(|a| serde_json::json!({
            "id": a.id,
            "name": a.name,
            "kind": a.kind,
            "active": file.active.as_deref() == Some(a.id.as_str()),
        })).collect::<Vec<_>>(),
        "active": file.active,
    });
    println!("\n=== 序列化后（前端会收到这个）===");
    println!("{}", serde_json::to_string_pretty(&view)?);

    // 模拟前端判断
    let active_str = view.get("active").and_then(|v| v.as_str());
    println!("\n=== 前端 find 结果 ===");
    for a in file.accounts.iter() {
        println!(
            "  id={} vs active={:?} → {}",
            a.id,
            active_str,
            active_str == Some(a.id.as_str())
        );
    }
    Ok(())
}
