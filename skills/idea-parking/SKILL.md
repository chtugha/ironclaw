---
name: idea-parking
version: 0.1.0
description: Park interesting ideas for later consideration, resurface them periodically, and promote to commitments when ready.
activation:
  keywords:
    - park this
    - save for later
    - interesting idea
    - maybe someday
    - backburner
    - idea for later
    - revisit later
    - shelve this
  patterns:
    - "(?i)(park|save|shelf|shelve|backburner) (this|that|the) (idea|thought)"
    - "(?i)not (now|yet|ready) but"
    - "(?i)(show|list|review) parked (ideas|items)"
    - "(?i)activate (idea|parked)"
  tags:
    - commitments
    - ideas
  max_context_tokens: 256
---

# Idea Parking

Park: write `projects/commitments/parked-ideas/<slug>.md` with type, parked_at, relevance, activation trigger. Confirm after write.

List: `memory_tree("projects/commitments/parked-ideas/", depth=1)` then `memory_read` each, show titles.

Promote: create commitment in `projects/commitments/open/`, clear parked file.

Dismiss: overwrite file with empty content.
