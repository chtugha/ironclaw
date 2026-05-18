---
name: new-project
version: 0.2.0
description: Create and structure a new autonomous project — "/new-project <what project does>"
activation:
  keywords:
    - project
    - create project
    - new project
    - set up project
    - autonomous workspace
  patterns:
    - "create a (new )?project"
    - "set up.*project"
    - "organize.*into.*project"
    - "/new.project"
  tags:
    - project-management
    - organization
    - goals
  max_context_tokens: 320
---

# New Project

Create an autonomous project workspace. Derive a slug (lowercase, hyphens).

Execute sequentially:
1. `memory_write("projects/{slug}/AGENTS.md", ...)` — agent instructions, specific and actionable
2. `memory_write("projects/{slug}/context.md", ...)` — overview, current state
3. `memory_write("projects/{slug}/goals.md", ...)` — measurable targets with metrics table if useful
4. `mission_create(name, goal, cadence, project_id="{slug}")` — recurring missions (daily/weekly/cron)

Structure: `projects/{slug}/AGENTS.md`, `context.md`, `goals.md`, `research/`, `reports/`

Rules: one tool call at a time; AGENTS.md first; always set `project_id` on missions.
