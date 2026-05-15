---
name: delegation-tracker
version: 0.1.0
description: Track delegated commitments, set follow-up timers, and generate nudge reminders when updates are overdue.
activation:
  keywords:
    - delegated
    - assigned to
    - waiting on
    - follow up with
    - check with
    - handed off
    - blocked on
    - nudge
  patterns:
    - "(?i)(delegated|assigned|handed off) (to|this to)"
    - "(?i)(waiting|blocked) on .+ (to|for)"
    - "(?i)follow up with .+ (about|on)"
    - "(?i)check (with|in with) .+ (about|on)"
  tags:
    - commitments
    - delegation
  max_context_tokens: 128
---

# Delegation Tracker

Track commitments waiting on others.

Delegation detected ("I asked X to...", "waiting on Y for..."):
1. Search `projects/commitments/open/` for existing
2. Write/update `open/<slug>.md`: status, delegated_to, expected date
3. Confirm: "Waiting on <person> for <topic> — flagging by <date>"

Update received: move to resolved/.
Triage flags overdue delegations automatically.
