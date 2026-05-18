---
name: notes
version: "1.0.0"
description: Store and retrieve local plain-text notes
activation:
  keywords:
    - "note"
    - "notes"
    - "remember"
    - "memo"
    - "jot"
    - "write down"
    - "remind me"
  patterns:
    - "(?i)(remember|note|jot).*(this|that|it)"
    - "(?i)(what).*(notes|noted)"
  tags:
    - "notes"
    - "local"
  max_context_tokens: 192
---

# Local Notes

Use the `local_notes` tool to store and recall information.

- `append_note` — add text to `~/.ironclaw/notes.md` (param: `text`)
- `read_notes` — read the full notes file
- `search_notes` — search notes by keyword (param: `query`)

Keep entries concise. One idea per note.
