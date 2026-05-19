//! Token budget enforcement for prompt assembly.
//!
//! [`apply`] degrades prompt content in priority order until the total token
//! count fits within the configured budget, or all droppable content has been
//! removed.
//!
//! Degradation order (REQ-2.4):
//! 1. Drop lowest-scoring memory docs (excluding `DocType::Plan`)
//! 2. Drop lowest-scoring skills
//! 3. Truncate dynamically-registered tool descriptions to 60 words
//! 4. Remove system prompt droppable sections (`<!-- droppable-start -->` /
//!    `<!-- droppable-end -->`)
//! 5. Drop oldest conversation history messages (never the latest user message)

use tracing::warn;

pub fn token_count(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    ((text.len() as f64 * 0.25) as usize).max(1)
}

/// Budget configuration for a single prompt assembly pass.
#[derive(Debug, Clone)]
pub struct PromptBudget {
    /// Hard ceiling on total tokens assembled.
    pub total: usize,
    /// Tokens reserved for the system prompt (informational, not enforced
    /// separately — the system prompt counts against `total`).
    pub system_prompt_reserved: usize,
    /// Maximum tokens available for all skills combined.
    /// Informational only — not yet enforced as a per-category sub-budget.
    /// Only `total` is tested during degradation; these fields exist for
    /// future per-category enforcement and for observability in callers.
    pub skill_budget: usize,
    /// Maximum tokens available for all memory docs combined.
    /// Informational only — not yet enforced as a per-category sub-budget.
    pub memory_doc_budget: usize,
    /// Maximum tokens available for tool schemas.
    /// Informational only — not yet enforced as a per-category sub-budget.
    pub tool_schema_budget: usize,
}

/// A skill or memory-doc item with a relevance score used for drop ordering.
#[derive(Debug, Clone)]
pub struct ScoredItem {
    /// Display name (skill name or doc_id string).
    pub name: String,
    /// Full text content that contributes to the token count.
    pub content: String,
    /// Relevance score — lower scores are dropped first.
    pub score: f64,
    /// Document type (for memory docs; empty string for skills).
    pub doc_type: String,
}

/// A conversation history message.
#[derive(Debug, Clone)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

/// All prompt parts that can be subject to budget degradation.
pub struct PromptParts {
    /// Assembled system prompt (already contains the plan anchor).
    pub system_prompt: String,
    /// Read-only copy of the plan anchor text for validation.
    pub plan_anchor_text: String,
    /// Candidate skills to inject (pre-scored and sorted by `select_skills`).
    pub skills: Vec<ScoredItem>,
    /// Candidate memory docs to inject (retrieved by `__retrieve_docs__`).
    pub memory_docs: Vec<ScoredItem>,
    /// Tool action schemas.
    pub tool_schemas: Vec<ScoredItem>,
    /// Conversation history (non-system messages).
    pub history: Vec<HistoryMessage>,
}

/// Summary of items that were dropped or truncated during budget enforcement.
#[derive(Debug, Default)]
pub struct DroppedItems {
    pub memory_docs: usize,
    pub skills: usize,
    pub tool_descriptions_truncated: usize,
    pub history_messages: usize,
}

/// Sentinel start tag for droppable system-prompt sections.
const DROPPABLE_START: &str = "<!-- droppable-start -->";
/// Sentinel end tag for droppable system-prompt sections.
const DROPPABLE_END: &str = "<!-- droppable-end -->";

/// Truncate `description` to at most 60 words.
pub fn truncate_to_60_words(description: &str) -> String {
    let mut iter = description.split_whitespace();
    let words: Vec<&str> = iter.by_ref().take(60).collect();
    if iter.next().is_none() {
        description.to_string()
    } else {
        words.join(" ")
    }
}

/// Remove sections wrapped in `<!-- droppable-start -->` / `<!-- droppable-end -->`
/// from the system prompt.
fn remove_droppable_sections(prompt: &str) -> String {
    let mut result = String::with_capacity(prompt.len());
    let mut remaining = prompt;
    loop {
        match remaining.find(DROPPABLE_START) {
            None => {
                result.push_str(remaining);
                break;
            }
            Some(start_idx) => {
                result.push_str(&remaining[..start_idx]);
                let after_start = &remaining[start_idx + DROPPABLE_START.len()..];
                match after_start.find(DROPPABLE_END) {
                    None => {
                        // Malformed — keep everything after the start tag as-is.
                        result.push_str(after_start);
                        break;
                    }
                    Some(end_idx) => {
                        remaining = &after_start[end_idx + DROPPABLE_END.len()..];
                    }
                }
            }
        }
    }
    result
}

/// Compute the total token count across all prompt parts.
fn total_tokens(parts: &PromptParts) -> usize {
    let mut total = token_count(&parts.system_prompt);
    for s in &parts.skills {
        total += token_count(&s.content);
    }
    for d in &parts.memory_docs {
        total += token_count(&d.content);
    }
    for t in &parts.tool_schemas {
        total += token_count(&t.content);
    }
    for h in &parts.history {
        total += token_count(&h.content);
    }
    total
}

/// Apply priority-order budget degradation to `parts`.
///
/// Returns a [`DroppedItems`] summary and modifies `parts` in place.
/// The caller should check `fits` (total ≤ budget after full degradation).
pub fn apply(budget: &PromptBudget, parts: &mut PromptParts, thread_id: &str) -> (DroppedItems, bool) {
    if !parts.plan_anchor_text.is_empty()
        && !parts.system_prompt.contains(&parts.plan_anchor_text)
    {
        warn!(
            thread_id = %thread_id,
            "plan_anchor not found in system_prompt — anchor may not have been injected"
        );
    }

    let mut dropped = DroppedItems::default();
    let mut current = total_tokens(parts);

    if current <= budget.total {
        return (dropped, true);
    }

    parts.memory_docs.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut i = 0;
    while i < parts.memory_docs.len() && current > budget.total {
        let doc_type_lower = parts.memory_docs[i].doc_type.to_lowercase();
        if doc_type_lower == "plan" {
            i += 1;
            continue;
        }
        let removed_tokens = token_count(&parts.memory_docs[i].content);
        parts.memory_docs.remove(i);
        current = current.saturating_sub(removed_tokens);
        dropped.memory_docs += 1;
    }

    if current <= budget.total {
        return (dropped, true);
    }

    parts.skills.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut skill_drop_count = 0;
    for skill in &parts.skills {
        if current <= budget.total {
            break;
        }
        let removed_tokens = token_count(&skill.content);
        current = current.saturating_sub(removed_tokens);
        skill_drop_count += 1;
    }
    if skill_drop_count > 0 {
        parts.skills.drain(..skill_drop_count);
        dropped.skills += skill_drop_count;
    }

    if current <= budget.total {
        return (dropped, true);
    }

    for schema in &mut parts.tool_schemas {
        let truncated = truncate_to_60_words(&schema.content);
        if truncated.len() < schema.content.len() {
            let old_tokens = token_count(&schema.content);
            schema.content = truncated;
            let new_tokens = token_count(&schema.content);
            current = current.saturating_sub(old_tokens.saturating_sub(new_tokens));
            dropped.tool_descriptions_truncated += 1;
            if current <= budget.total {
                return (dropped, true);
            }
        }
    }

    if current <= budget.total {
        return (dropped, true);
    }

    let new_prompt = remove_droppable_sections(&parts.system_prompt);
    if new_prompt.len() < parts.system_prompt.len() {
        let old_tokens = token_count(&parts.system_prompt);
        let new_tokens = token_count(&new_prompt);
        parts.system_prompt = new_prompt;
        current = current.saturating_sub(old_tokens.saturating_sub(new_tokens));
        if !parts.plan_anchor_text.is_empty()
            && !parts.system_prompt.contains(&parts.plan_anchor_text)
        {
            warn!(
                thread_id = %thread_id,
                "plan_anchor lost after droppable section removal — anchor was inside a droppable block"
            );
        }
    }

    if current <= budget.total {
        return (dropped, true);
    }

    while !parts.history.is_empty() && current > budget.total {
        let last_user_idx = parts
            .history
            .iter()
            .rposition(|m| m.role.to_lowercase() == "user");
        let drop_idx = (0..parts.history.len()).find(|&idx| Some(idx) != last_user_idx);
        match drop_idx {
            Some(idx) => {
                let removed_tokens = token_count(&parts.history[idx].content);
                parts.history.remove(idx);
                current = current.saturating_sub(removed_tokens);
                dropped.history_messages += 1;
            }
            None => break,
        }
    }

    let fits = current <= budget.total;
    (dropped, fits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_budget(total: usize) -> PromptBudget {
        PromptBudget {
            total,
            system_prompt_reserved: 0,
            skill_budget: total,
            memory_doc_budget: total,
            tool_schema_budget: total,
        }
    }

    fn scored_item(name: &str, content: &str, score: f64) -> ScoredItem {
        ScoredItem {
            name: name.to_string(),
            content: content.to_string(),
            score,
            doc_type: String::new(),
        }
    }

    fn plan_doc(name: &str, content: &str, score: f64) -> ScoredItem {
        ScoredItem {
            name: name.to_string(),
            content: content.to_string(),
            score,
            doc_type: "Plan".to_string(),
        }
    }

    #[test]
    fn budget_exactly_satisfied_no_drops() {
        // system_prompt of ~4 tokens (16 bytes * 0.25)
        let system = "hello world test ok";  // 19 bytes → 4 tokens
        let mut parts = PromptParts {
            system_prompt: system.to_string(),
            plan_anchor_text: String::new(),
            skills: vec![],
            memory_docs: vec![],
            tool_schemas: vec![],
            history: vec![],
        };
        // Budget exactly matches the token count.
        let budget = small_budget(token_count(system));
        let (dropped, fits) = apply(&budget, &mut parts, "test-thread");
        assert!(fits);
        assert_eq!(dropped.memory_docs, 0);
        assert_eq!(dropped.skills, 0);
        assert_eq!(dropped.history_messages, 0);
    }

    #[test]
    fn memory_docs_dropped_when_over_budget() {
        // 4-word system prompt ≈ tiny
        let mut parts = PromptParts {
            system_prompt: "hi".to_string(),
            plan_anchor_text: String::new(),
            skills: vec![],
            memory_docs: vec![
                scored_item("doc-low", &"x".repeat(100), 0.1),
                scored_item("doc-high", &"x".repeat(100), 0.9),
            ],
            tool_schemas: vec![],
            history: vec![],
        };
        // Budget only fits system prompt.
        let budget = small_budget(2);
        let (dropped, _fits) = apply(&budget, &mut parts, "t");
        assert!(dropped.memory_docs > 0, "should have dropped memory docs");
    }

    #[test]
    fn skills_dropped_after_memory_docs() {
        let base = "hi";
        let mut parts = PromptParts {
            system_prompt: base.to_string(),
            plan_anchor_text: String::new(),
            skills: vec![scored_item("skill-a", &"x".repeat(200), 0.5)],
            memory_docs: vec![scored_item("doc-a", &"x".repeat(50), 0.5)],
            tool_schemas: vec![],
            history: vec![],
        };
        // Small budget — forces both memory doc and skill to be dropped.
        let budget = small_budget(1);
        let (dropped, _fits) = apply(&budget, &mut parts, "t");
        assert!(dropped.memory_docs > 0 || dropped.skills > 0);
    }

    #[test]
    fn plan_doc_never_dropped() {
        let mut parts = PromptParts {
            system_prompt: "hi".to_string(),
            plan_anchor_text: String::new(),
            skills: vec![],
            memory_docs: vec![
                plan_doc("active-plan", &"x".repeat(400), 0.0),
                scored_item("regular-doc", &"x".repeat(400), 0.9),
            ],
            tool_schemas: vec![],
            history: vec![],
        };
        let budget = small_budget(1);
        let (dropped, _fits) = apply(&budget, &mut parts, "t");
        // The plan doc must still be present.
        assert!(
            parts.memory_docs.iter().any(|d| d.name == "active-plan"),
            "plan doc must not be dropped"
        );
        // The regular doc should be dropped.
        assert!(dropped.memory_docs >= 1);
    }

    #[test]
    fn plan_memory_docs_excluded_from_drop_candidates() {
        let mut parts = PromptParts {
            system_prompt: "hi".to_string(),
            plan_anchor_text: String::new(),
            skills: vec![],
            memory_docs: vec![
                plan_doc("plan-1", &"p".repeat(300), 0.0),
                plan_doc("plan-2", &"p".repeat(300), 0.1),
            ],
            tool_schemas: vec![],
            history: vec![],
        };
        let budget = small_budget(1);
        let (dropped, _fits) = apply(&budget, &mut parts, "t");
        assert_eq!(
            dropped.memory_docs, 0,
            "plan-type docs must not be dropped"
        );
        assert_eq!(
            parts.memory_docs.len(),
            2,
            "both plan docs must remain"
        );
    }

    #[test]
    fn droppable_system_prompt_sections_removed() {
        let system = "keep this <!-- droppable-start -->drop this<!-- droppable-end --> keep too";
        let mut parts = PromptParts {
            system_prompt: system.to_string(),
            plan_anchor_text: String::new(),
            skills: vec![scored_item("sk", &"x".repeat(400), 0.5)],
            memory_docs: vec![scored_item("doc", &"x".repeat(400), 0.5)],
            tool_schemas: vec![],
            history: vec![],
        };
        // Very tight budget so steps 1-2 don't fully fix it, reaching step 4.
        let budget = small_budget(1);
        let _ = apply(&budget, &mut parts, "t");
        assert!(
            !parts.system_prompt.contains("drop this"),
            "droppable section should have been removed"
        );
        assert!(
            parts.system_prompt.contains("keep this"),
            "non-droppable content must remain"
        );
    }
}
