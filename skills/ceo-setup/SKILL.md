---
name: ceo-setup
version: 0.4.0
description: One-time setup for the executive commitment workflow — delegation, decisions, digests.
activation:
  setup_marker: projects/commitments/.ceo-setup-complete
  keywords:
    - ceo assistant
    - executive assistant
    - manager assistant
    - delegation setup
    - leadership workflow
  patterns:
    - "(?i)I'm a (CEO|manager|executive|director|VP|founder)"
    - "(?i)set ?up.*(executive|manager|leadership|delegation)"
    - "(?i)help me manage my (day|schedule|team)"
  tags:
    - commitments
  max_context_tokens: 320
requires:
  skills:
    - commitment-triage
    - commitment-digest
    - decision-capture
    - delegation-tracker
---

# CEO/Manager Setup

One-time setup for the executive commitment workflow.

1. Create `projects/commitments/` with: open/, resolved/, decisions/, parked-ideas/, tech-debt/
2. Write `projects/commitments/AGENTS.md` with commitment tracking instructions
3. Create daily triage mission: review open commitments, flag overdue
4. Create morning brief mission: summarize pending decisions and delegations
5. Write setup marker: `projects/commitments/.ceo-setup-complete`
