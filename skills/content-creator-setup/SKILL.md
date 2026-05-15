---
name: content-creator-setup
version: 0.1.0
description: One-time setup for local content planning and creation workflow.
activation:
  setup_marker: projects/content/.content-creator-setup-complete
  keywords:
    - content creator
    - content setup
    - content workflow
    - setup content
    - content planning
    - blog setup
    - writing workflow
  patterns:
    - "(?i)set ?up (content|blog|writing) (workflow|system|workspace)"
    - "(?i)I (create|write|produce) (content|articles|posts)"
  tags:
    - content
  max_context_tokens: 128
---

# Content Creator Setup

One-time setup for local content planning and creation.

1. Create `projects/content/` with: ideas/, drafts/, published/, calendar.md
2. Write `projects/content/AGENTS.md` with content creation instructions
3. Create weekly content planning mission (review ideas, draft top picks)
4. Create monthly content calendar mission
5. Write setup marker: `projects/content/.content-creator-setup-complete`
