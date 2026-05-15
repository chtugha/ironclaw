---
name: developer-setup
version: 0.1.0
description: One-time setup for the developer workflow — code review, triage, daily brief.
activation:
  setup_marker: projects/commitments/.developer-setup-complete
  keywords:
    - developer setup
    - dev setup
    - setup developer
    - dev workflow
    - developer workflow
    - coding setup
  patterns:
    - "(?i)set ?up (developer|dev|coding) (workflow|workspace|assistant)"
    - "(?i)I('m| am) a (developer|programmer|engineer|coder)"
    - "(?i)help me (manage|track|organize) (my )?(code|repos|PRs)"
  tags:
    - developer
  max_context_tokens: 128
requires:
  skills:
    - code-review
    - qa-review
    - security-review
    - tech-debt-tracker
---

# Developer Setup

One-time setup for the developer workflow.

1. Create `projects/commitments/` workspace (open, resolved, tech-debt, decisions)
2. Write `projects/commitments/AGENTS.md` — developer tracking instructions
3. Create daily dev brief mission: open PRs, CI status, tech debt backlog
4. Create weekly triage mission: stale PRs, overdue items
5. Write setup marker: `projects/commitments/.developer-setup-complete`
