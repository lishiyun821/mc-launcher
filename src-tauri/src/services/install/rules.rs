use crate::core::models::{OsRule, Rule};

/// 当前平台在 Mojang 规则里的名字
pub fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

/// 当前 CPU 架构在 Mojang 规则里的名字
pub fn os_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86"
    }
}

/// Mojang 的 rules 语义：
/// - 空数组 = 放行
/// - 按顺序遍历，命中的规则把结果设为它的 action（allow/deny），后命中的覆盖先命中的
/// - 一条都没命中 = 拒绝
pub fn allows(rules: &[Rule]) -> bool {
    if rules.is_empty() {
        return true;
    }

    let mut allowed = false;
    for rule in rules {
        if !rule_matches(rule) {
            continue;
        }
        allowed = rule.action == "allow";
    }
    allowed
}

fn rule_matches(rule: &Rule) -> bool {
    // 依赖启动参数的 features（demo / customResolution / quickPlay 等）。
    // M1 阶段没有这些特性，一律视为不匹配 —— 与「没开 demo 模式」的真实情况一致。
    if rule.features.is_some() {
        return false;
    }

    match &rule.os {
        None => true,
        Some(os) => os_matches(os),
    }
}

fn os_matches(os: &OsRule) -> bool {
    // 版本号规则（只有极老的 launcher 用过），一律不匹配
    if os.version.is_some() {
        return false;
    }

    let name_ok = os
        .name
        .as_deref()
        .map(|n| n == os_name())
        .unwrap_or(true);

    let arch_ok = os
        .arch
        .as_deref()
        .map(|a| a == os_arch())
        .unwrap_or(true);

    name_ok && arch_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(action: &str, os_name: Option<&str>) -> Rule {
        Rule {
            action: action.to_string(),
            os: os_name.map(|n| OsRule {
                name: Some(n.to_string()),
                arch: None,
                version: None,
            }),
            features: None,
        }
    }

    #[test]
    fn empty_rules_allow() {
        assert!(allows(&[]));
    }

    #[test]
    fn plain_allow() {
        assert!(allows(&[rule("allow", None)]));
    }

    #[test]
    fn deny_only_current_os() {
        // [allow 全部, deny windows] → 在 Windows 上拒绝
        let rules = vec![rule("allow", None), rule("deny", Some("windows"))];
        assert_eq!(allows(&rules), !cfg!(target_os = "windows"));
    }

    #[test]
    fn allow_only_other_os() {
        // [allow linux] → 在 Windows 上不命中 → 拒绝
        let rules = vec![rule("allow", Some("linux"))];
        assert_eq!(allows(&rules), cfg!(target_os = "linux"));
    }
}
