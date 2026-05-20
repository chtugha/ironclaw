---
title: Planning Pipeline
description: How IronClaw breaks down goals before executing them
---

IronClaw uses a **plan-first execution model** designed for local LLMs with limited context windows. Before starting any tool calls, the agent creates an explicit step-by-step plan. This keeps smaller models on track and makes multi-step tasks reliable.

---

## Why Plan First?

Local LLMs with 7–14B parameters tend to lose track of complex goals mid-execution. A common failure pattern is:

1. User asks: "Set up a Python project with tests and a Makefile"
2. The model creates the Python files, then forgets about the Makefile
3. The model declares success without completing the task

The planning pipeline prevents this by:

- Making the goal explicit as a numbered list of steps before any tool is called
- Tracking which step is current and including it in every subsequent prompt
- Detecting when a model has gone off-track and re-grounding it

---

## Planning Pipeline Overview

```
User message
     │
     ▼
┌─────────────────────────────────────────────────────┐
│                  run_planning_phase()                │
│                                                     │
│  1. Trivial check → single-step plan               │
│  2. Check cached plan (above confidence threshold) │
│  3. Check plan template (authoritative match)      │
│  4. Call LLM for minimal plan (≤ 200 tokens)       │
│  5. If None → decompose into 2–4 subtasks          │
└─────────────────────────────────────────────────────┘
     │
     ▼ (steps, source)
┌─────────────────────────────────────────────────────┐
│                     run_loop()                      │
│                                                     │
│  For each step:                                     │
│    - Token guard check                              │
│    - LLM reasoning with plan anchor in prompt      │
│    - Tool calls                                     │
│    - Advance plan_current_step                     │
│    - Check for plan invalidation signals           │
└─────────────────────────────────────────────────────┘
```

---

## Plan Sources

The planning phase tries five sources in order. The first successful source wins.

### 1. Trivial goals

Goals that are short (≤ 8 words), contain a `?`, or match known single-step patterns (e.g. "what is", "list", "show me") skip the planning LLM call and use a single-step plan: `["complete the request"]`.

### 2. Cached plan (runtime memory)

If IronClaw has executed a similar goal before, it retrieves the plan from memory and reuses it — if the stored plan's confidence score meets the threshold (default 0.6).

```toml
[agent]
plan_confidence_threshold = 0.6   # 0.0–1.0; lower = reuse more aggressively
```

Confidence is updated after each execution:

```
confidence = 1.0 - (failure_count / (execution_count + 1))
```

### 3. Plan template

The `docs/internal/plan-templates/` directory contains seed templates for common tasks (installing software, web searches, calendar events, etc.). Templates take priority over runtime cached plans — they are authoritative blueprints that always win over learned patterns.

**Included templates:**

| Template | Triggers |
|---------|---------|
| `install-software.md` | install, setup, configure |
| `web-search.md` | search, find online, browse |
| `calendar-events.md` | meeting, calendar, schedule |
| `notes.md` | note, remember, jot |
| `file-search.md` | find file, search files, locate |
| `git-operations.md` | git, commit, push, branch |
| `system-info.md` | disk space, memory, CPU, processes |

### 4. LLM planning call

If no cached plan or template matches, IronClaw calls the LLM with a minimal 2-message prompt (≤ 200 tokens) asking it to produce a numbered step list. The response is parsed into steps.

### 5. Goal decomposition

If the minimal planning call produces no parseable steps (or the token guard determines the budget is too tight even for a stripped prompt), IronClaw decomposes the goal:

1. Calls the LLM with a 2–4 subtask decomposition prompt
2. Runs each subtask as a separate mini-loop
3. Passes the last response (≤ 200 token summary) to the next subtask as context

Decomposition is limited to one level deep — subtasks are always executed directly.

---

## Plan Anchor

During execution, the current plan is shown in the prompt as a **plan anchor**:

```markdown
## Current Plan
1. Create the project directory structure
**→ 2. Write the Python module files** (current)
3. Create test files
4. Write the Makefile
```

The `→` marker shows the current step. This keeps the model focused even when tool call results are long.

The plan anchor is capped at 200 tokens and is always preserved by the token guard (never dropped).

---

## Plan Invalidation

When the user sends a new message mid-execution, IronClaw checks whether the new message invalidates the current plan. Phrases that trigger plan invalidation:

- "instead", "forget", "stop", "cancel", "actually", "new task", "switch to"
- "do this instead", "change of plan", "never mind", "start over"

When invalidated, the current plan is abandoned, the plan doc is marked as failed, and a new planning phase starts with the new goal.

---

## Plan Confidence Tracking

Each time a plan is used, its execution count and failure count are updated:

| Outcome | Effect |
|---------|-------|
| `completed` | `execution_count += 1` |
| `failed` / `stopped` | `execution_count += 1`, `failure_count += 1` |

Confidence degrades with failures and recovers with successes. Plans with confidence below `plan_confidence_threshold` are not reused from cache.

---

## Tuning the Planning Pipeline

```toml
[agent]
plan_confidence_threshold = 0.6   # raise to require higher confidence before reusing a plan
```

To disable plan reuse entirely (always plan fresh):

```toml
[agent]
plan_confidence_threshold = 1.0   # only reuse plans that have never failed
```

To make the planner more aggressive about reuse (useful when your model plans well):

```toml
[agent]
plan_confidence_threshold = 0.3
```

---

## Adding Custom Plan Templates

Create a file in `docs/internal/plan-templates/` with YAML frontmatter:

```markdown
---
title: Deploy a Docker container
keywords:
  - docker deploy
  - container run
  - docker-compose up
tags:
  - devops
  - docker
confidence: 0.9
is_template: true
---

1. Check that Docker is running and the image exists
2. Pull the latest image if needed
3. Stop and remove the existing container
4. Run the new container with the correct environment
5. Verify the container is healthy
```

Templates are loaded at startup. Restart IronClaw after adding a template.

---

## Debugging

Enable debug logging to trace planning decisions:

```bash
RUST_LOG=ironclaw_engine::executor=debug IRONCLAW_PROFILE=local ironclaw
```

Log lines to look for:

```
[planning] source=template confidence=0.9 steps=5
[planning] source=cached confidence=0.72 steps=3
[planning] source=llm steps=4
[planning] source=decompose subtasks=3
[token_guard] fits=false → decompose
```
