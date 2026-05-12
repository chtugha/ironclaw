use crate::executor::prompt::{PlatformInfo, TIER0_SYSTEM_PROMPT_MARKER};

const TIER0_SYSTEM_PROMPT_TEMPLATE: &str =
    include_str!("../../prompts/tier0_system_prompt.md");

/// Returns true if the execution backend should use the Tier 0 compact system prompt
/// instead of the full CodeAct prompt.
///
/// Decision logic:
/// - If `codeact_override` is `Some(true)`, CodeAct is explicitly enabled → use CodeAct.
/// - If `codeact_override` is `Some(false)`, CodeAct is explicitly disabled → use Tier 0.
/// - If `codeact_override` is `None`, fall back to `platform.is_local_backend`.
pub fn should_use_tier0(
    platform: Option<&PlatformInfo>,
    codeact_override: Option<bool>,
) -> bool {
    match codeact_override {
        Some(true) => false,
        Some(false) => true,
        None => platform.map(|p| p.is_local_backend).unwrap_or(false),
    }
}

/// Build the Tier 0 compact system prompt.
///
/// Reads `tier0_system_prompt.md` (embedded at compile time), prepends the
/// `TIER0_SYSTEM_PROMPT_MARKER`, renders the `{plan_anchor}` placeholder, and
/// injects optional platform identity.
pub fn build_tier0_system_prompt(
    platform: Option<&PlatformInfo>,
    plan_anchor: Option<&str>,
) -> String {
    let template = TIER0_SYSTEM_PROMPT_TEMPLATE;

    let plan_anchor_section = plan_anchor
        .filter(|s| !s.is_empty())
        .map(|anchor| format!("\n{anchor}\n"))
        .unwrap_or_default();

    let body = template.replace("{plan_anchor}", &plan_anchor_section);

    let mut prompt = String::from(TIER0_SYSTEM_PROMPT_MARKER);

    if let Some(info) = platform {
        let platform_section = info.to_prompt_section();
        if !platform_section.is_empty() {
            prompt.push_str(&platform_section);
            prompt.push('\n');
        }
    } else {
        prompt.push_str("You are IronClaw, a local AI assistant.\n\n");
    }

    prompt.push_str(&body);
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tier0_system_prompt_contains_marker() {
        let prompt = build_tier0_system_prompt(None, None);
        assert!(prompt.starts_with(TIER0_SYSTEM_PROMPT_MARKER));
    }

    #[test]
    fn build_tier0_system_prompt_token_budget() {
        let prompt = build_tier0_system_prompt(None, None);
        let token_estimate = (prompt.len() as f64 * 0.25) as usize;
        assert!(
            token_estimate <= 1024,
            "Tier 0 prompt estimated at {token_estimate} tokens, exceeds 1024 token limit"
        );
    }

    #[test]
    fn build_tier0_system_prompt_includes_plan_anchor() {
        let anchor = "## Current Plan\n1. Search for files\n**→ 2. Summarize results** (current)";
        let prompt = build_tier0_system_prompt(None, Some(anchor));
        assert!(prompt.contains(anchor));
    }

    #[test]
    fn build_tier0_system_prompt_no_plan_anchor_when_empty() {
        let prompt_none = build_tier0_system_prompt(None, None);
        let prompt_empty = build_tier0_system_prompt(None, Some(""));
        assert_eq!(prompt_none, prompt_empty);
    }

    #[test]
    fn should_use_tier0_explicit_codeact_enabled() {
        assert!(!should_use_tier0(None, Some(true)));

        let local_platform = PlatformInfo {
            is_local_backend: true,
            ..Default::default()
        };
        assert!(!should_use_tier0(Some(&local_platform), Some(true)));
    }

    #[test]
    fn should_use_tier0_explicit_codeact_disabled() {
        assert!(should_use_tier0(None, Some(false)));

        let remote_platform = PlatformInfo {
            is_local_backend: false,
            ..Default::default()
        };
        assert!(should_use_tier0(Some(&remote_platform), Some(false)));
    }

    #[test]
    fn should_use_tier0_auto_detects_local_backend() {
        let local_platform = PlatformInfo {
            is_local_backend: true,
            ..Default::default()
        };
        assert!(should_use_tier0(Some(&local_platform), None));

        let remote_platform = PlatformInfo {
            is_local_backend: false,
            ..Default::default()
        };
        assert!(!should_use_tier0(Some(&remote_platform), None));
    }

    #[test]
    fn should_use_tier0_no_platform_no_override_returns_false() {
        assert!(!should_use_tier0(None, None));
    }
}
