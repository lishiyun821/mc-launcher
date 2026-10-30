//! 微软账户登录（设备码流程 + Xbox Live + Minecraft Services）
//!
//! ## 为什么用 `login.live.com` 而不是 `login.microsoftonline.com`
//!
//! 开源启动器必须让**任何下载了它的人都能登录**，不能要求每个用户去 Azure 注册应用。
//! `login.live.com` 的旧版设备码接口接受 Xbox 官方的公共 client_id
//! （`00000000402b5328`），这是 Microsoft 公开的公共客户端，无需 client_secret，
//! 也是 HMCL 等开源启动器采用的方式。
//!
//! ## 完整链路
//!
//! ```text
//! ① devicecode   login.live.com/oauth20_connect.srf → device_code + user_code
//! ② 用户浏览器    https://www.microsoft.com/link 输入 user_code
//! ③ token        login.live.com/oauth20_token.srf  → MS access_token + refresh_token
//! ④ XBL          user.auth.xboxlive.com            → XBL token + uhs
//! ⑤ XSTS         xsts.auth.xboxlive.com            → XSTS token + uhs + xuid
//! ⑥ MC 登录       api.minecraftservices.com        → Minecraft access_token
//! ⑦ 档案          /minecraft/profile               → uuid + name + 皮肤
//! ```

use serde::{Deserialize, Serialize};
use serde_json::json;

/// Xbox 官方公共客户端 id（公开信息，所有开源启动器都在用）
pub const DEFAULT_CLIENT_ID: &str = "00000000402b5328";

/// Minecraft 专用的 scope
const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";

const CONNECT_URL: &str = "https://login.live.com/oauth20_connect.srf";
const TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/* ─────────────────────────── ① 设备码 ─────────────────────────── */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub device_code: String,
    /// 给用户看的短码，例如 `ZLHD5BTZ`
    pub user_code: String,
    /// 用户在浏览器里打开的地址
    pub verification_uri: String,
    /// 有效期（秒），通常 900
    pub expires_in: u64,
    /// 轮询间隔（秒），通常 5
    pub interval: u64,
    /// 微软给的完整提示文案
    #[serde(default)]
    pub message: String,
}

#[derive(Deserialize)]
struct DeviceCodeRaw {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    #[serde(default = "default_interval")]
    interval: u64,
    #[serde(default)]
    message: String,
}

fn default_interval() -> u64 {
    5
}

/// 申请设备码
pub async fn request_device_code(http: &reqwest::Client) -> Result<DeviceCode, anyhow::Error> {
    let res = http
        .post(CONNECT_URL)
        .form(&[
            ("client_id", DEFAULT_CLIENT_ID),
            ("scope", SCOPE),
            ("response_type", "device_code"),
        ])
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("连不上微软登录服务器: {e}"))?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "申请登录码失败（HTTP {}）: {}",
            status.as_u16(),
            short(&text)
        ));
    }

    let raw: DeviceCodeRaw = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("登录码格式异常: {e}"))?;

    Ok(DeviceCode {
        device_code: raw.device_code,
        user_code: raw.user_code,
        verification_uri: raw.verification_uri,
        expires_in: raw.expires_in,
        interval: raw.interval.max(3),
        message: raw.message,
    })
}

/* ─────────────────────────── ③ 轮询令牌 ─────────────────────────── */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MsToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

/// 一次轮询的结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum PollResult {
    /// 用户还没在浏览器里完成
    Pending,
    /// 用户完成了，拿到令牌
    Success(MsToken),
    /// 微软要求放慢轮询
    SlowDown,
    /// 出错（含用户拒绝、码过期）
    Failed(String),
}

#[derive(Deserialize)]
struct TokenRaw {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

/// 轮询一次（前端每隔 interval 秒调一次）
pub async fn poll_token(
    http: &reqwest::Client,
    device_code: &str,
) -> Result<PollResult, anyhow::Error> {
    let res = http
        .post(TOKEN_URL)
        .form(&[
            ("client_id", DEFAULT_CLIENT_ID),
            ("device_code", device_code),
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:device_code",
            ),
        ])
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("轮询失败: {e}"))?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    let raw: TokenRaw = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => {
            return Ok(PollResult::Failed(format!(
                "令牌响应无法解析（HTTP {}）",
                status.as_u16()
            )))
        }
    };

    if let (Some(access), Some(refresh)) = (raw.access_token.clone(), raw.refresh_token.clone()) {
        return Ok(PollResult::Success(MsToken {
            access_token: access,
            refresh_token: refresh,
            expires_in: raw.expires_in.unwrap_or(3600),
        }));
    }

    let err = raw.error.unwrap_or_default();

    Ok(match err.as_str() {
        "authorization_pending" => PollResult::Pending,
        "slow_down" => PollResult::SlowDown,
        "expired_token" => PollResult::Failed("登录码已过期，请重新生成".to_string()),
        "authorization_declined" => PollResult::Failed("你在浏览器里拒绝了授权".to_string()),
        "bad_refresh_token" => PollResult::Failed("刷新令牌失效，请重新登录".to_string()),
        "" => PollResult::Failed(format!("未知响应（HTTP {}）", status.as_u16())),
        other => PollResult::Failed(format!(
            "{other}: {}",
            raw.error_description.unwrap_or_default()
        )),
    })
}

/// 用 refresh_token 换新的 MS 令牌（免登录续期）
pub async fn refresh_ms_token(
    http: &reqwest::Client,
    refresh_token: &str,
) -> Result<MsToken, anyhow::Error> {
    let res = http
        .post(TOKEN_URL)
        .form(&[
            ("client_id", DEFAULT_CLIENT_ID),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
            ("scope", SCOPE),
        ])
        .send()
        .await?;

    let text = res.text().await.unwrap_or_default();
    let raw: TokenRaw = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("刷新响应异常: {e}"))?;

    match (raw.access_token, raw.refresh_token) {
        (Some(a), Some(r)) => Ok(MsToken {
            access_token: a,
            refresh_token: r,
            expires_in: raw.expires_in.unwrap_or(3600),
        }),
        _ => Err(anyhow::anyhow!(
            "刷新失败: {}",
            raw.error_description.unwrap_or_default()
        )),
    }
}

/* ─────────────────────────── ④⑤ XBL / XSTS ─────────────────────────── */

#[derive(Debug, Clone)]
pub struct XstsResult {
    pub token: String,
    pub uhs: String,
    pub xuid: String,
}

#[derive(Deserialize)]
struct XblRaw {
    #[serde(rename = "Token")]
    token: Option<String>,
    #[serde(rename = "DisplayClaims")]
    claims: Option<Claims>,
}

#[derive(Deserialize)]
struct Claims {
    xui: Option<Vec<Xui>>,
}

#[derive(Deserialize)]
struct Xui {
    uhs: Option<String>,
    xid: Option<String>,
}

/// XBL 认证
async fn xbl_auth(http: &reqwest::Client, ms_access: &str) -> Result<(String, String), anyhow::Error> {
    let body = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={ms_access}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let res = http
        .post(XBL_URL)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Xbox Live 认证连不上: {e}"))?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Xbox Live 认证失败（HTTP {}）: {}",
            status.as_u16(),
            short(&text)
        ));
    }

    let raw: XblRaw = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("XBL 响应异常: {e}"))?;

    let token = raw.token.ok_or_else(|| anyhow::anyhow!("XBL 没有返回 token"))?;
    let uhs = raw
        .claims
        .and_then(|c| c.xui)
        .and_then(|x| x.into_iter().next())
        .and_then(|x| x.uhs)
        .unwrap_or_default();

    Ok((token, uhs))
}

/// XSTS 授权
async fn xsts_auth(http: &reqwest::Client, xbl_token: &str) -> Result<XstsResult, anyhow::Error> {
    let body = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let res = http
        .post(XSTS_URL)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("XSTS 授权连不上: {e}"))?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();

    if !status.is_success() {
        // 401 带 XErr 是最常见的两种情况，翻译成人话
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(xerr) = v.get("XErr").and_then(|x| x.as_u64()) {
                let msg = match xerr {
                    2148916233 => "这个微软账号还没有 Xbox 账号，请先去 xbox.com 创建一个",
                    2148916235 => "这个账号所在地区不支持 Xbox Live",
                    2148916236 | 2148916237 => "账号需要完成成人验证",
                    2148916238 => "这是未成年账号，需要家长在 Microsoft 家庭里同意后才能登录",
                    _ => "XSTS 授权被拒绝",
                };
                return Err(anyhow::anyhow!("{msg}（错误码 {xerr}）"));
            }
        }
        return Err(anyhow::anyhow!(
            "XSTS 授权失败（HTTP {}）: {}",
            status.as_u16(),
            short(&text)
        ));
    }

    let raw: XblRaw = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("XSTS 响应异常: {e}"))?;

    let token = raw
        .token
        .ok_or_else(|| anyhow::anyhow!("XSTS 没有返回 token"))?;

    let xui = raw
        .claims
        .and_then(|c| c.xui)
        .and_then(|x| x.into_iter().next());

    Ok(XstsResult {
        token,
        uhs: xui.as_ref().and_then(|x| x.uhs.clone()).unwrap_or_default(),
        xuid: xui.and_then(|x| x.xid).unwrap_or_default(),
    })
}

/* ─────────────────────────── ⑥ 换 Minecraft 令牌 ─────────────────────────── */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McToken {
    pub access_token: String,
    pub expires_in: u64,
}

async fn minecraft_login(
    http: &reqwest::Client,
    uhs: &str,
    xsts_token: &str,
) -> Result<McToken, anyhow::Error> {
    let body = json!({
        "identityToken": format!("XBL3.0 x={uhs};{xsts_token}")
    });

    let res = http
        .post(MC_LOGIN_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Minecraft 登录连不上: {e}"))?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Minecraft 登录失败（HTTP {}）: {}",
            status.as_u16(),
            short(&text)
        ));
    }

    #[derive(Deserialize)]
    struct Raw {
        access_token: Option<String>,
        expires_in: Option<u64>,
    }

    let raw: Raw = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Minecraft 登录响应异常: {e}"))?;

    Ok(McToken {
        access_token: raw
            .access_token
            .ok_or_else(|| anyhow::anyhow!("Minecraft 没有返回令牌"))?,
        expires_in: raw.expires_in.unwrap_or(86400),
    })
}

/* ─────────────────────────── ⑦ 玩家档案 ─────────────────────────── */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// 无横线 UUID
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<SkinEntry>,
    #[serde(default)]
    pub capes: Vec<CapeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub variant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapeEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub alias: String,
}

/// 拉取档案（顺带验证令牌是否有效、账号是否拥有游戏）
pub async fn fetch_profile(
    http: &reqwest::Client,
    mc_token: &str,
) -> Result<Profile, anyhow::Error> {
    let res = http
        .get(MC_PROFILE_URL)
        .bearer_auth(mc_token)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("拉取玩家档案失败: {e}"))?;

    let status = res.status();

    if status.as_u16() == 404 {
        return Err(anyhow::anyhow!(
            "这个账号没有购买 Minecraft: Java Edition"
        ));
    }
    if status.as_u16() == 401 {
        return Err(anyhow::anyhow!("Minecraft 令牌无效，请重新登录"));
    }

    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "拉取档案失败（HTTP {}）: {}",
            status.as_u16(),
            short(&text)
        ));
    }

    serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("档案格式异常: {e}"))
}

/* ─────────────────────────── 一条龙 ─────────────────────────── */

/// 登录成功后的全部结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginOutcome {
    pub ms: MsToken,
    pub profile: Profile,
    pub minecraft_token: McToken,
    pub xuid: String,
}

/// 从 MS 令牌一路走到玩家档案
pub async fn login_with_ms_token(
    http: &reqwest::Client,
    ms: MsToken,
) -> Result<LoginOutcome, anyhow::Error> {
    let (xbl_token, _xbl_uhs) = xbl_auth(http, &ms.access_token).await?;
    let xsts = xsts_auth(http, &xbl_token).await?;
    let mc = minecraft_login(http, &xsts.uhs, &xsts.token).await?;
    let profile = fetch_profile(http, &mc.access_token).await?;

    Ok(LoginOutcome {
        ms,
        profile,
        minecraft_token: mc,
        xuid: xsts.xuid,
    })
}

fn short(s: &str) -> String {
    let cut: String = s.chars().take(200).collect();
    cut.replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_code_shape() {
        // 只验证结构能被序列化成前端要的 camelCase
        let d = DeviceCode {
            device_code: "dc".into(),
            user_code: "ABCD1234".into(),
            verification_uri: "https://www.microsoft.com/link".into(),
            expires_in: 900,
            interval: 5,
            message: "hi".into(),
        };
        let s = serde_json::to_string(&d).unwrap();
        assert!(s.contains("\"userCode\":\"ABCD1234\""));
        assert!(s.contains("\"verificationUri\""));
    }

    #[test]
    fn default_client_id_is_public_xbox_app() {
        assert_eq!(DEFAULT_CLIENT_ID, "00000000402b5328");
    }
}
