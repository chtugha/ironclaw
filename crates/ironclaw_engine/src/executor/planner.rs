#[derive(Debug, Clone, PartialEq)]
pub struct PlanAnchor {
    pub steps: Vec<String>,
    pub current_step: usize,
}

impl PlanAnchor {
    const MAX_TOKENS: usize = 200;
    const BYTES_PER_TOKEN: f64 = 4.0;

    /// Format the plan as a numbered list for injection into a Tier 0 system prompt.
    /// Current step is highlighted. Output is truncated to ≤ 200 tokens (byte approximation).
    /// When truncation is needed, a window centered around the current step is shown
    /// so the active step is always visible.
    pub fn to_prompt_section(&self) -> String {
        if self.steps.is_empty() {
            return String::new();
        }

        let max_bytes = (Self::MAX_TOKENS as f64 * Self::BYTES_PER_TOKEN) as usize;
        let current_step = self.current_step.min(self.steps.len().saturating_sub(1));

        let step_lines: Vec<String> = self
            .steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let step_num = i + 1;
                if i == current_step {
                    format!("**→ {step_num}. {step}** (current)")
                } else {
                    format!("{step_num}. {step}")
                }
            })
            .collect();

        let header = "## Current Plan";
        let mut lines = vec![header.to_string()];
        lines.extend(step_lines.iter().cloned());

        let full_text = lines.join("\n");
        if full_text.len() <= max_bytes {
            return full_text;
        }

        let ellipsis = "…";
        let ellipsis_line_cost = 1 + ellipsis.len();
        let current_idx = current_step;
        let current_line = &step_lines[current_idx];

        let mut budget = header.len() + 1 + current_line.len();
        if current_idx > 0 {
            budget += ellipsis_line_cost;
        }
        if current_idx + 1 < step_lines.len() {
            budget += ellipsis_line_cost;
        }

        let mut before: Vec<usize> = Vec::new();
        let mut after: Vec<usize> = Vec::new();

        let mut lo = current_idx;
        let mut hi = current_idx;
        loop {
            let can_go_before = lo > 0;
            let can_go_after = hi + 1 < step_lines.len();
            if !can_go_before && !can_go_after {
                break;
            }

            let mut made_progress = false;

            if can_go_before {
                let candidate = &step_lines[lo - 1];
                let needed = candidate.len() + 1;
                if budget + needed <= max_bytes {
                    budget += needed;
                    lo -= 1;
                    before.push(lo);
                    if lo == 0 {
                        budget -= ellipsis_line_cost;
                    }
                    made_progress = true;
                }
            }

            if can_go_after {
                let candidate = &step_lines[hi + 1];
                let needed = candidate.len() + 1;
                if budget + needed <= max_bytes {
                    budget += needed;
                    hi += 1;
                    after.push(hi);
                    if hi + 1 == step_lines.len() {
                        budget -= ellipsis_line_cost;
                    }
                    made_progress = true;
                }
            }

            if !made_progress {
                break;
            }
        }

        before.sort();

        let mut result = vec![header.to_string()];
        if lo > 0 {
            result.push(ellipsis.to_string());
        }
        for &idx in &before {
            result.push(step_lines[idx].clone());
        }
        result.push(current_line.clone());
        for &idx in &after {
            result.push(step_lines[idx].clone());
        }
        if hi + 1 < step_lines.len() {
            result.push(ellipsis.to_string());
        }

        result.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_prompt_section_highlights_current_step() {
        let anchor = PlanAnchor {
            steps: vec![
                "Search for files".to_string(),
                "Summarize results".to_string(),
                "Return answer".to_string(),
            ],
            current_step: 1,
        };
        let output = anchor.to_prompt_section();
        assert!(output.contains("**→ 2. Summarize results** (current)"));
        assert!(output.contains("1. Search for files"));
        assert!(output.contains("3. Return answer"));
    }

    #[test]
    fn to_prompt_section_fits_within_200_tokens_for_10_steps() {
        let steps: Vec<String> = (1..=10)
            .map(|i| format!("Step {i}: do some meaningful work here"))
            .collect();
        let anchor = PlanAnchor {
            steps,
            current_step: 0,
        };
        let output = anchor.to_prompt_section();
        let token_estimate = (output.len() as f64 * 0.25) as usize;
        assert!(
            token_estimate <= 200,
            "output estimated at {token_estimate} tokens, exceeds 200 limit"
        );
    }

    #[test]
    fn to_prompt_section_fits_within_200_tokens_for_20_long_steps() {
        let steps: Vec<String> = (1..=20)
            .map(|i| format!("Step {i}: this is a very detailed description of a task that needs to be completed as part of the overall plan"))
            .collect();
        let anchor = PlanAnchor {
            steps,
            current_step: 0,
        };
        let output = anchor.to_prompt_section();
        let token_estimate = (output.len() as f64 * 0.25) as usize;
        assert!(
            token_estimate <= 200,
            "output estimated at {token_estimate} tokens, exceeds 200 limit"
        );
    }

    #[test]
    fn to_prompt_section_empty_steps_returns_empty() {
        let anchor = PlanAnchor {
            steps: vec![],
            current_step: 0,
        };
        assert_eq!(anchor.to_prompt_section(), "");
    }

    #[test]
    fn to_prompt_section_current_step_always_visible_when_deep() {
        let steps: Vec<String> = (1..=20)
            .map(|i| {
                format!("Step {i}: this is a very detailed description of a task that needs to be completed")
            })
            .collect();
        let anchor = PlanAnchor {
            steps,
            current_step: 17,
        };
        let output = anchor.to_prompt_section();
        assert!(
            output.contains("(current)"),
            "Current step must be visible even when deep in a long plan. Output:\n{output}"
        );
        assert!(
            output.contains("**→ 18."),
            "Step 18 (index 17) must be highlighted. Output:\n{output}"
        );
        let token_estimate = (output.len() as f64 * 0.25) as usize;
        assert!(
            token_estimate <= 200,
            "output estimated at {token_estimate} tokens, exceeds 200 limit"
        );
    }

    #[test]
    fn to_prompt_section_current_step_out_of_bounds_clamps() {
        let anchor = PlanAnchor {
            steps: vec!["Only step".to_string()],
            current_step: 99,
        };
        let output = anchor.to_prompt_section();
        assert!(output.contains("(current)"));
    }

    #[test]
    fn to_prompt_section_both_ellipses_stay_within_budget() {
        let steps: Vec<String> = (1..=30)
            .map(|i| format!("Step {i}: perform a moderately detailed task"))
            .collect();
        let anchor = PlanAnchor {
            steps,
            current_step: 15,
        };
        let output = anchor.to_prompt_section();
        assert!(output.contains("(current)"));
        assert!(output.contains("…"));
        let token_estimate = (output.len() as f64 * 0.25) as usize;
        assert!(
            token_estimate <= 200,
            "output with both ellipses estimated at {token_estimate} tokens, exceeds 200 limit. Byte len: {}",
            output.len()
        );
    }
}
