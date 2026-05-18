---
name: commitment-digest
version: 0.1.0
description: Generate a brief summary of open commitments — urgency, overdue, delegations.
activation:
  keywords:
    - commitment digest
    - daily digest
    - morning brief
    - open commitments
    - pending commitments
    - what's pending
    - commitment summary
  patterns:
    - "(?i)(show|list|summarize).*(commitments|obligations)"
    - "(?i)(morning|daily).*(brief|digest|summary)"
  tags:
    - commitments
  max_context_tokens: 256
---

# Commitment Digest

Generate a summary of open commitments.

1. `memory_tree("projects/commitments/open/", depth=1)` — list open items
2. `memory_read` each: extract urgency, due date, owner, delegated_to
3. Group by urgency; flag overdue (due < today)
4. Present: "X open commitments — Y urgent, Z overdue"

For full detail, include title and next action. Keep digest under 200 words.
