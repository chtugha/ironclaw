---
name: review-readiness
version: 0.1.0
description: PR readiness dashboard — tracks which reviews have been completed per branch and gates merge decisions.
activation:
  keywords:
    - review readiness
    - ready to merge
    - PR readiness
    - review status
    - can I merge
    - ready to ship
  patterns:
    - "(?i)(is|are) (this|it|PR) ready (to|for) (merge|ship)"
    - "(?i)(review|merge|ship) (readiness|checklist|status)"
    - "(?i)what (checks|reviews) are (missing|left|needed)"
  tags:
    - developer
    - review
    - process
  max_context_tokens: 128
---

# Review Readiness

Track PR readiness in `projects/<owner>-<repo>/readiness/<branch>.md`. Checks: code review, tests, security, QA, linting.

Verdict: READY (all done, no P1/P2, CI green) | ALMOST READY | NOT READY | BLOCKED (P1 or CI failing).

When asked "is this ready?": read/create readiness file, show status, list missing checks with suggestions (`/security-review`, `/qa-review`).
