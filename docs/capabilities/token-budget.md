---
title: Token Budget
description: How IronClaw fits within a local LLM's context window
---

Local LLMs have a hard context limit — typically 4 096 to 32 768 tokens. IronClaw enforces a configurable prompt token budget and degrades gracefully when the assembled prompt would exceed it.

---

## Budget Defaults

| Setting | Default | Profile: `server` |
|---------|---------|------------------|
| `agent.max_prompt_tokens` | 8 192 | 131 072 |
| `skills.max_context_tokens` | 2 048 | 2 048 |

The skill budget is always a subset of the total prompt budget. Setting `max_context_tokens > max_prompt_tokens` is rejected with a validation error.

---

## How the Budget is Spent

Every prompt assembled by IronClaw has five parts, listed in priority order (highest = kept last):

```
┌──────────────────────────────── max_prompt_tokens = 8192 ────┐
│ System prompt     (≤ 800 tokens for Tier 0 / local LLMs)     │
│ Plan anchor       (≤ 200 tokens — current step summary)       │
│ Skill context     (≤ 2048 tokens — bundled SKILL.md content)  │
│ Memory docs       (variable — retrieved workspace notes)      │
│ Conversation      (remainder — oldest messages dropped first) │
└──────────────────────────────────────────────────────────────┘
```

---

## Token Guard — Graceful Degradation

When the assembled prompt would exceed `max_prompt_tokens`, the **Token Guard** drops content in this order:

1. **Low-scoring memory docs** — documents with the lowest relevance score are dropped first. Plan documents are never dropped.
2. **Low-scoring skills** — skill context from the least-relevant skill is dropped.
3. **Tool description text** — each tool's description is truncated to 60 words. The tool itself remains callable; only its documentation is trimmed.
4. **Droppable system-prompt sections** — sections of the system prompt marked with `<!-- droppable-start -->` / `<!-- droppable-end -->` are removed.
5. **Old conversation history** — the oldest messages are removed. The most recent user message and any plan anchor text are never removed.

If the prompt still does not fit after all drops, the system triggers **goal decomposition** — the goal is split into smaller subtasks, each executed with a fresh context.

---

## Configuration

### Via `config.toml`

```toml
# ~/.ironclaw/config.toml

[agent]
max_prompt_tokens = 8192          # total context budget for all content
plan_confidence_threshold = 0.6  # min confidence to reuse a cached plan

[skills]
max_context_tokens = 2048         # skill injection budget (≤ max_prompt_tokens)
```

### Via Settings UI

Go to **Settings → Agent** in the web UI:

- **Max total prompt tokens** — hard limit for the assembled prompt
- **Skill context token size** — maximum tokens consumed by skill injection

### Via environment variable

```env
# Not directly exposed as env vars — use config.toml or the Settings UI
```

---

## Adjusting for Your Model

### Consumer GPU (6–12 GB VRAM)

| Model | Context | Recommended budget |
|-------|---------|-------------------|
| Llama 3.2 3B | 128 K | 4 096–8 192 |
| Qwen 2.5 7B | 128 K | 8 192 |
| Qwen 2.5 14B | 128 K | 8 192–16 384 |
| Phi-4 14B | 16 K | 8 192 |
| Mistral 7B | 32 K | 8 192 |

Even though these models support large context windows, **IronClaw defaults to 8 192** because:
- Most tasks complete well within this budget
- Token generation is proportional to context length — larger contexts slow responses
- Skill injection quality is better with a tighter budget (only the most relevant content gets in)

### Larger GPU (24+ GB VRAM) or high-RAM CPU

If you have a 32B or 70B model with a large context:

```toml
[agent]
max_prompt_tokens = 32768

[skills]
max_context_tokens = 4096
```

### Server deployment

The `server` profile sets `max_prompt_tokens = 131072` by default, suitable for cloud models with 128K+ context.

---

## Token Counting

IronClaw uses a byte-based approximation for token counting:

```
tokens ≈ bytes × 0.25
```

This is accurate for English/Latin text (roughly 4 bytes per token). For multilingual text (CJK, Arabic), the approximation may under-count — keep a safety margin of 10–20% if working with non-Latin content.

> **Future improvement:** CJK/Arabic-aware counting with `tiktoken` is planned (Step 22 in the roadmap).

---

## Monitoring Budget Usage

IronClaw logs budget decisions at the `debug` level:

```bash
RUST_LOG=ironclaw_engine=debug IRONCLAW_PROFILE=local ironclaw
```

Look for log lines like:

```
[token_guard] budget=8192 assembled=9104 dropped: memory_docs=2 skills=1 history=3
[token_guard] fits=true remaining=1201
```

---

## Planning and Goal Decomposition

When the token guard determines that even a stripped-down prompt does not fit the budget (at step 0, before any tools run), IronClaw triggers **goal decomposition**:

1. The current goal is split into 2–4 subtasks by a minimal LLM call (≤ 200 tokens budget)
2. Each subtask is executed in sequence with its own fresh context
3. Subtask results are carried forward via the `_last_response` field (≤ 200 token summary)

This allows complex goals to be completed even on very constrained models.

See [Planning Pipeline](/capabilities/planning) for the full plan-first execution model.
