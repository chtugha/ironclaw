---
name: commitment-triage
version: 0.1.0
description: Review open commitments for action — flags overdue, stale delegations, and untracked decisions.
activation:
  keywords:
    - triage
    - commitment triage
    - review commitments
    - overdue commitments
    - stale delegations
    - flag overdue
  patterns:
    - "(?i)(triage|review) (open )?commitments"
    - "(?i)what (commitments|obligations) (are )?overdue"
    - "(?i)(check|review) (my )?(tasks|delegations|commitments)"
  tags:
    - commitments
  max_context_tokens: 128
---

# Commitment Triage

Review open commitments for action.

1. `memory_tree("projects/commitments/open/")` — list all
2. Flag overdue (due < today, status: open/waiting)
3. Flag stale delegations (delegated_to set, no update in 3+ days)
4. Flag decisions 7+ days old without outcome
5. Present: overdue first, then stale, then no-update items
6. Ask user to resolve, defer, or dismiss each flagged item
