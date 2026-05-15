---
name: commitment-setup
version: 0.1.0
description: One-time setup for the personal commitment tracking workspace.
activation:
  setup_marker: projects/commitments/.commitment-setup-complete
  keywords:
    - commitment setup
    - setup commitments
    - track commitments
    - commitment workspace
    - setup tracking
  patterns:
    - "(?i)set ?up (commitment|obligation|task) (tracking|workspace|system)"
    - "(?i)I want to track (my )?(commitments|obligations|tasks)"
  tags:
    - commitments
  max_context_tokens: 128
requires:
  skills:
    - commitment-triage
    - commitment-digest
---

# Commitment Setup

One-time setup for personal commitment tracking.

1. Create `projects/commitments/` with: open/, resolved/, decisions/, parked-ideas/, tech-debt/
2. Write `projects/commitments/AGENTS.md` — commitment tracking instructions
3. Write `projects/commitments/open/README.md` — format guide
4. Create weekly digest mission and triage mission
5. Write setup marker: `projects/commitments/.commitment-setup-complete`
