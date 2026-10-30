use serde::{Deserialize, Serialize};

/// 启动用的账户（离线或正版）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// 游戏内名字，3-16 字符
    pub name: String,
    /// 无横线 UUID（32 位小写十六进制）
    pub uuid: String,
    /// 访问令牌，离线模式填占位符
    pub access_token: String,
    /// 账户类型：msa / legacy / offline
    pub user_type: String,
    /// 正版账户的 xuid（离线留空）
    #[serde(default)]
    pub xuid: String,
}

impl Account {
    /// 用玩家名生成离线账户
    ///
    /// UUID 算法与 Java 的 `UUID.nameUUIDFromBytes("OfflinePlayer:" + name)` 一致（MD5 + 版本 3 位处理），
    /// 这样同一个名字在任何启动器里拿到的 UUID 都相同，存档和服务器白名单不会错乱。
    pub fn offline(name: &str) -> Self {
        Self {
            name: name.to_string(),
            uuid: offline_uuid(name),
            // 离线模式没有真令牌，但要给个非空值，否则部分版本会报鉴权失败
            access_token: "0".repeat(32),
            user_type: "legacy".to_string(),
            xuid: String::new(),
        }
    }

    /// 从已保存的正版账户构造
    pub fn from_stored(a: &crate::services::auth::StoredAccount) -> Self {
        Self {
            name: a.name.clone(),
            uuid: a.id.clone(),
            access_token: a.access_token.clone(),
            user_type: "msa".to_string(),
            xuid: a.xuid.clone().unwrap_or_default(),
        }
    }

    /// 带横线的 UUID（8-4-4-4-12）
    pub fn uuid_dashed(&self) -> String {
        format_uuid(&self.uuid)
    }
}

/// 离线 UUID：MD5("OfflinePlayer:<name>") 再按 RFC 4122 v3 规整
pub fn offline_uuid(name: &str) -> String {
    // md5 0.7 的 API：compute 直接返回 Digest（内部是 [u8; 16]）
    let digest = md5::compute(format!("OfflinePlayer:{name}").as_bytes());
    let mut d = digest.0;

    // 版本位：v3
    d[6] = (d[6] & 0x0f) | 0x30;
    // variant 位：RFC 4122
    d[8] = (d[8] & 0x3f) | 0x80;

    hex::encode(d)
}

/// 32 位无横线 → 8-4-4-4-12
fn format_uuid(raw: &str) -> String {
    let s: String = raw.chars().filter(|c| *c != '-').collect();
    if s.len() != 32 {
        return raw.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &s[0..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..32]
    )
}

/// 校验玩家名是否合法（Minecraft 规则：3-16 位，字母数字下划线）
pub fn validate_name(name: &str) -> Result<(), String> {
    let n = name.trim();
    if n.len() < 3 || n.len() > 16 {
        return Err("名字长度要在 3-16 个字符之间".to_string());
    }
    if !n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("名字只能包含字母、数字和下划线".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_matches_java() {
        // Java: UUID.nameUUIDFromBytes("OfflinePlayer:Notch".getBytes())
        // 已知结果：b50ad385-829d-3141-a216-7e7d7539ba7f
        assert_eq!(offline_uuid("Notch"), "b50ad385829d3141a2167e7d7539ba7f");
    }

    #[test]
    fn dashed_format() {
        assert_eq!(
            format_uuid("b50ad385829d3141a2167e7d7539ba7f"),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
    }

    #[test]
    fn name_validation() {
        assert!(validate_name("Steve").is_ok());
        assert!(validate_name("ab").is_err());
        assert!(validate_name("bad name").is_err());
        assert!(validate_name("a".repeat(17).as_str()).is_err());
    }
}
