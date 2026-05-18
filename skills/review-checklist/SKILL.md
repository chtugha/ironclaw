---
name: review-checklist
version: 0.1.0
description: Pre-merge review checklist based on recurring AI reviewer feedback patterns
activation:
  patterns:
    - "review.*checklist"
    - "ready to merge"
    - "pre-merge check"
    - "check.*before.*merge"
  keywords:
    - review
    - checklist
    - merge
    - pre-merge
  max_context_tokens: 256
---

# Pre-Merge Checklist

- Multi-step DB ops wrapped in transactions; both postgres+libsql backends updated
- Tool params redacted before logging; URL validation resolves DNS before IP check
- Destructive tools require approval; no secrets in logs or error messages
- No byte-index slicing on user strings; file extension comparisons case-insensitive
- New trait methods delegated in ALL wrapper types
- Temp files use `tempfile` crate; tests use no real network; test names match behavior
