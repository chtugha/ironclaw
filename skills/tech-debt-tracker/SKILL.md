---
name: tech-debt-tracker
version: 0.1.0
description: Detect and track technical debt from conversation. Resurface in weekly retros, promote to commitments when ready.
activation:
  keywords:
    - tech debt
    - technical debt
    - hack
    - hacky
    - refactor later
    - fixme
    - workaround
    - shortcut
    - should refactor
    - bandaid
    - temporary fix
    - debt backlog
  patterns:
    - "(?i)(this|that) is (a )?(hack|workaround|bandaid|kludge)"
    - "(?i)(we |I )should (refactor|clean up|rewrite) (this|that) (later|someday)"
    - "(?i)(show|list|review) (tech )?debt"
    - "(?i)add.*tech ?debt"
  tags:
    - commitments
    - developer
    - tech-debt
  max_context_tokens: 128
---

# Tech Debt Tracker

Detect: "this is a hack", "should refactor later", "TODO: fix" → write `projects/commitments/tech-debt/<slug>.md` with type, severity, category, proposed fix.

List: `memory_tree("projects/commitments/tech-debt/", depth=1)`, show grouped by severity.

Resolve: move to `projects/commitments/resolved/`.

Promote: create commitment in `projects/commitments/open/`.
