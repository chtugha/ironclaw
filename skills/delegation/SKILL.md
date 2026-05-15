---
name: delegation
version: 0.1.0
description: Helps users delegate tasks, break them into steps, set deadlines, and track progress.
activation:
  keywords:
    - delegate
    - hand off
    - assign task
    - take care of
    - remind me to
    - follow up on
  patterns:
    - "can you.*handle"
    - "I need (help|someone) to"
    - "take over"
    - "set up a reminder"
    - "follow up on"
  tags:
    - personal-assistant
    - task-management
    - delegation
  max_context_tokens: 128
---

# Task Delegation

1. Clarify: what, by when, constraints (skip if clear)
2. Break down: write `tasks/{slug}.md` with steps and due date via `memory_write`
3. Track: `routine_create` for recurring check-ins; set up reminder if deadline exists
4. Respect profile: check `USER.md` for proactivity level and communication style
5. Execute or queue: do now if possible, else schedule reminder
6. Confirm plan with user before starting; update task file and notify on completion
