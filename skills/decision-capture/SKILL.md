---
name: decision-capture
version: 0.2.0
description: Detect decisions in conversation and record them with rationale, alternatives, and outcome tracking.
activation:
  keywords:
    - decided
    - decision
    - chose
    - going with
    - settled on
    - picked
    - landed on
    - went with
    - finalized
    - agreed on
    - opted for
    - concluded
    - record decision
  exclude_keywords:
    - undecided
    - considering
    - tentative
    - maybe
  patterns:
    - "(?i)(we|I|team) (decided|chose|went with|picked|settled on|opted for)"
    - "(?i)let's go with"
    - "(?i)the (decision|call) is"
    - "(?i)record (this|that) decision"
  tags:
    - commitments
    - decision-making
  max_context_tokens: 320
---

# Decision Capture

When a clear decision is expressed ("we decided X", "going with Y"), write to `projects/commitments/decisions/<date>-<slug>.md` via `memory_write` with: type, decided_at, context, confidence, reversible, options, rationale.

Do NOT capture brainstorming or hypotheticals. Ask if uncertain.

If decision creates an obligation, also create `projects/commitments/open/<slug>.md`.

Confirm only after writes succeed.
