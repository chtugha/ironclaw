---
name: plan-mode
version: 0.1.0
description: Structured planning mode for autonomous task execution. Creates plans as MemoryDocs, executes via Missions, tracks progress with live checklist.
activation:
  keywords:
    - "[PLAN MODE]"
    - plan mode
    - create a plan
    - make a plan
    - execution plan
    - step by step plan
  patterns:
    - "\\[PLAN MODE\\]"
    - "plan (out|how to|before|for)"
  tags:
    - planning
    - autonomous
    - task-management
  max_context_tokens: 256
---

# Plan Mode

Structured planning and autonomous execution via MemoryDocs and Missions.

## Creating a plan ([PLAN MODE] Create)
1. `memory_search` for relevant prior work
2. Write plan to `plans/<slug>.md`: goal, success criteria, numbered steps with tools/risk/est, risks section
3. `plan_update(status="draft", steps=[...])`
4. Tell user: "Use `/plan approve` to start or `/plan revise <slug> <feedback>`"

Steps format: `1. [ ] Title -- tools: [tool] -- risk: low -- est: 5min`
Keep under 20 steps; decompose larger work.

## Approving ([PLAN MODE] Approve)
`mission_create(name="plan:<slug>", goal=<plan content>, cadence="manual")` → `mission_fire` → `plan_update(status="executing")`

## During execution
Check `current_focus`, execute step, `plan_update` (mark done, next in_progress). On failure: try once, then mark failed and stop.

## Status ([PLAN MODE] Show status)
`memory_search` for plan, `mission_list` for state, summarize X/Y steps done.
