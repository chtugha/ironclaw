<p align="center">
  <img src="ironclaw.png?v=2" alt="IronClaw" width="200"/>
</p>

<h1 align="center">IronClaw</h1>

<p align="center">
  <strong>Your secure personal AI assistant — runs entirely on your hardware</strong>
</p>

<p align="center">
  <a href="#license"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache%202.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
  <a href="https://t.me/ironclawAI"><img src="https://img.shields.io/badge/Telegram-%40ironclawAI-26A5E4?style=flat&logo=telegram&logoColor=white" alt="Telegram: @ironclawAI" /></a>
  <a href="https://www.reddit.com/r/ironclawAI/"><img src="https://img.shields.io/badge/Reddit-r%2FironclawAI-FF4500?style=flat&logo=reddit&logoColor=white" alt="Reddit: r/ironclawAI" /></a>
</p>

<p align="center">
  <a href="README.md">English</a> |
  <a href="README.zh-CN.md">简体中文</a> |
  <a href="README.ru.md">Русский</a> |
  <a href="README.ja.md">日本語</a> |
  <a href="README.ko.md">한국어</a>
</p>

<p align="center">
  <a href="#philosophy">Philosophy</a> •
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#local-llm-setup">Local LLM Setup</a> •
  <a href="#configuration">Configuration</a> •
  <a href="#security">Security</a> •
  <a href="#architecture">Architecture</a>
</p>

---

## Philosophy

IronClaw is built on a simple principle: **your AI assistant should work for you, not against you**.

In a world where AI systems increasingly depend on cloud subscriptions, rate limits, and opaque data handling, IronClaw takes a different approach:

- **100% local operation** — runs on your own hardware with Ollama or any OpenAI-compatible server; no cloud account required
- **Your data stays yours** — all information stored locally, encrypted, never leaves your control
- **Transparency by design** — open source, auditable, no telemetry or data harvesting
- **Fits consumer hardware** — tuned to work within an 8 192-token context window; models as small as 7B work well
- **Defense in depth** — multiple security layers protect against prompt injection and data exfiltration

IronClaw is the AI assistant you can actually trust with your personal and professional life.

---

## Features

### Home-Use Optimised

- **Token-aware engine** — hard budget of 8 192 total prompt tokens; automatically trims skill context, memory docs, and history to fit any local model
- **Plan-first execution** — breaks complex goals into steps before acting, so smaller models stay on track
- **Tier 0 system prompt** — compact ≤ 800-token system prompt designed for local LLMs; no CodeAct overhead
- **Local tools bundled** — browser (Playwright MCP), CalDAV calendar, plain-text notes, and workspace file search included out of the box
- **Skill budgets** — each skill declares its token cost; zero-budget skills are never injected

### Security First

- **WASM Sandbox** — untrusted tools run in isolated WebAssembly containers with capability-based permissions
- **Credential Protection** — secrets are never exposed to tools; injected at the host boundary with leak detection
- **Prompt Injection Defense** — pattern detection, content sanitisation, and policy enforcement
- **Endpoint Allowlisting** — HTTP requests only to explicitly approved hosts and paths
- **`file://` URL blocking** — the browser tool rejects local file URLs; use the `local_search` tool for filesystem access

### Always Available

- **Multi-channel** — REPL/TUI, HTTP webhooks, web gateway, and WASM channels (Telegram, Slack, Discord)
- **Missions / Routines** — cron schedules, idle triggers, and event handlers for background automation
- **Parallel threads** — handle multiple requests concurrently with isolated contexts
- **Persistent memory** — hybrid full-text + vector search with Reciprocal Rank Fusion

### Self-Expanding

- **MCP Protocol** — connect to any Model Context Protocol server (local or remote)
- **Dynamic Tool Building** — describe what you need; IronClaw builds it as a WASM tool
- **Plugin Architecture** — drop in new WASM tools and channels without restarting

---

## Quick Start

**Minimum requirements:** any machine with ~8 GB RAM, a modern 64-bit CPU, and about 4 GB of free disk space for models.

### Step 1 — Install IronClaw

Choose the option that matches your OS:

<details>
<summary>macOS or Linux (shell installer)</summary>

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-installer.sh | sh
```

</details>

<details>
<summary>macOS (Homebrew)</summary>

```bash
brew install ironclaw
```

</details>

<details>
<summary>Windows (PowerShell)</summary>

```powershell
irm https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-installer.ps1 | iex
```

Or download the [Windows Installer (.msi)](https://github.com/nearai/ironclaw/releases/latest/download/ironclaw-x86_64-pc-windows-msvc.msi) and run it directly.

</details>

<details>
<summary>Build from source (Rust / Cargo)</summary>

Requires [Rust 1.92+](https://rustup.rs).

```bash
git clone https://github.com/nearai/ironclaw.git
cd ironclaw
cargo build --release
# Binary at: ./target/release/ironclaw
```

</details>

### Step 2 — Install Ollama and pull a model

[Ollama](https://ollama.com) runs local LLMs with a single command. Download it from [ollama.com/download](https://ollama.com/download), then pull a model:

```bash
# Small and fast — good starting point (4 GB download)
ollama pull llama3.2

# Better reasoning — needs ~8 GB VRAM or 16 GB RAM (5 GB download)
ollama pull qwen2.5:14b

# Confirm Ollama is running
ollama list
```

> **GPU note:** Ollama automatically uses an NVIDIA, AMD (ROCm), or Apple Silicon GPU if available. CPU inference works but is slower.

### Step 3 — Run IronClaw for the first time

```bash
# Use the home profile (libSQL embedded database, no Postgres needed)
IRONCLAW_PROFILE=local ironclaw
```

On first run IronClaw will ask a few questions:

1. **Database** — press Enter to accept the default `libSQL` embedded database (no installation required)
2. **LLM provider** — select `ollama` and enter the model name you pulled (e.g. `llama3.2`)
3. **Secrets storage** — select `system keychain` (recommended) or `env var` for headless environments

That's it. Start chatting:

```
> Hello! What can you do?
```

---

## Local LLM Setup

### Ollama (recommended for home use)

```bash
# Install Ollama: https://ollama.com/download
ollama serve           # starts on http://localhost:11434 (auto-starts on macOS)

# Pull your chosen model
ollama pull llama3.2           # 3B, fast, 2 GB
ollama pull qwen2.5:14b        # 14B, balanced, 9 GB
ollama pull qwen2.5:32b        # 32B, best quality, 20 GB
```

Environment variables (or set them with `ironclaw onboard`):

```env
LLM_BACKEND=ollama
OLLAMA_MODEL=llama3.2
# OLLAMA_BASE_URL=http://localhost:11434   # default, change if Ollama is on another machine
```

IronClaw automatically detects that Ollama is a local backend and switches to the **Tier 0 compact system prompt** — a ≤ 800-token preamble designed for small context windows.

### LM Studio

Start LM Studio, load a model, and enable the local server (default port 1234):

```env
LLM_BACKEND=openai_compatible
LLM_BASE_URL=http://localhost:1234/v1
LLM_MODEL=llama-3.2-3b-instruct-q4_K_M
```

### Any OpenAI-compatible server (vLLM, llama.cpp, Kobold, etc.)

```env
LLM_BACKEND=openai_compatible
LLM_BASE_URL=http://localhost:8000/v1
LLM_API_KEY=none          # required field but value is ignored when auth is disabled
LLM_MODEL=my-model-name
```

IronClaw auto-detects loopback addresses (`localhost`, `127.0.0.1`, `[::1]`) and treats the provider as local, applying the same compact prompt.

### Model recommendations

| Model | Size | VRAM / RAM | Notes |
|-------|------|-----------|-------|
| `llama3.2` | 3B | 4 GB | Fast, good for simple tasks |
| `qwen2.5:7b` | 7B | 6 GB | Good reasoning, recommended minimum |
| `qwen2.5:14b` | 14B | 10 GB | Best quality within 8192 tokens |
| `qwen2.5:32b` | 32B | 20 GB | Near-cloud quality locally |
| `phi4` | 14B | 10 GB | Strong at coding and reasoning |
| `mistral` | 7B | 6 GB | Fast, good instruction following |

For context: IronClaw's default token budget is **8 192 total tokens** (including 2 048 for skill context). All models above fit comfortably.

---

## Configuration

### Profile-based setup

IronClaw ships with ready-made profiles. Set `IRONCLAW_PROFILE` in your environment or in `~/.ironclaw/.env`:

| Profile | Best for | Database | Sandbox |
|---------|---------|---------|---------|
| `local` | Home use (default) | libSQL embedded | Disabled |
| `local-sandbox` | Home use + Docker isolation | libSQL embedded | Enabled |
| `server` | Single-user server | PostgreSQL | Enabled |
| `server-multitenant` | Multi-user server | PostgreSQL | Enabled |

```bash
# ~/.ironclaw/.env
IRONCLAW_PROFILE=local
```

### Configuration file

Fine-tune settings in `~/.ironclaw/config.toml`:

```toml
[agent]
max_prompt_tokens = 8192          # total context budget
plan_confidence_threshold = 0.6  # reuse a cached plan if confidence ≥ this value

[skills]
max_context_tokens = 2048         # skill injection budget (subset of max_prompt_tokens)

[local_search]
allow_global_scope = false        # set true to allow searching outside workspace
```

### Environment variables

All settings can also be set as environment variables. Key variables:

```env
# LLM backend
LLM_BACKEND=ollama
OLLAMA_MODEL=llama3.2

# Profile
IRONCLAW_PROFILE=local

# Database (libSQL default — no URL needed for local profile)
# DATABASE_URL=postgres://user:pass@localhost/ironclaw  # for server profile

# Web gateway (optional, disabled in local profile)
# GATEWAY_ENABLED=true
# GATEWAY_PORT=3000
# GATEWAY_AUTH_TOKEN=change-me

# Embedding model for semantic memory search (optional)
# EMBEDDING_ENABLED=true
# EMBEDDING_MODEL=nomic-embed-text   # via Ollama
```

See [`docs/capabilities/configuration.mdx`](docs/capabilities/configuration.mdx) for the full reference.

### Token budget settings

For local LLMs the token budget is critical. IronClaw enforces hard limits:

| Setting | Default | Description |
|---------|---------|-------------|
| `agent.max_prompt_tokens` | 8 192 | Total prompt tokens (system + history + skills + tools) |
| `skills.max_context_tokens` | 2 048 | Skill injection budget (must be ≤ `max_prompt_tokens`) |

The **Token Guard** automatically drops content in priority order when the budget is exceeded:
1. Low-scoring memory docs (keeping plan docs)
2. Low-scoring skills
3. Tool description text (truncated to 60 words)
4. Droppable system-prompt sections
5. Old conversation history (newest user message always kept)

See [`docs/capabilities/token-budget.md`](docs/capabilities/token-budget.md) for details.

---

## Local Tools

IronClaw bundles four local tools enabled by default (`home` bundle):

| Tool | Description | Setup |
|------|-------------|-------|
| **Browser** (Playwright MCP) | Navigate websites, extract content, take screenshots | Requires `node` + `npx` |
| **CalDAV** | Read/write calendar events and todos | Requires `CALDAV_URL`, credentials |
| **Local Notes** | Append and search a personal notes file at `~/.ironclaw/notes.md` | Zero-config |
| **Local Search** | Search files within the current workspace | Zero-config |

```bash
# Install the browser tool's runtime
npm install -g playwright
npx playwright install chromium
```

See [`docs/extensions/local-tools.md`](docs/extensions/local-tools.md) for full setup instructions.

---

## Skills

Skills are markdown files injected into the LLM context when relevant keywords are detected. All bundled skills are trimmed to fit local model budgets:

| Category | Budget cap | Examples |
|----------|-----------|---------|
| Behavioral guidance | ≤ 128 tokens | `commit`, `code-review`, `security-review` |
| Tool/procedure | ≤ 256 tokens | `coding`, `local-test`, `new-project` |
| Local services | ≤ 256 tokens | `caldav`, `notes`, `local-search`, `web-browse` |
| Cloud (opt-in) | varies | `github`, `linear` (require credentials) |

Skills with `max_context_tokens: 0` are never injected. A startup warning is emitted for any skill missing a declared budget.

```bash
ironclaw skills list       # show all loaded skills with their budgets
ironclaw skills show git   # show a specific skill's content
```

See [`docs/capabilities/skills.mdx`](docs/capabilities/skills.mdx) for authoring your own skills.

---

## Security

IronClaw implements defence in depth to protect your data and prevent misuse.

### WASM Sandbox

All untrusted tools run in isolated WebAssembly containers:

- **Capability-based permissions** — explicit opt-in for HTTP, secrets, tool invocation
- **Endpoint allowlisting** — HTTP requests only to approved hosts/paths
- **Credential injection** — secrets injected at host boundary, never exposed to WASM code
- **Leak detection** — scans requests and responses for secret exfiltration attempts
- **Rate limiting** — per-tool request limits to prevent abuse
- **`file://` URL blocking** — browser tool rejects local file access; use `local_search` instead

```
WASM ──► Allowlist ──► Leak Scan ──► Credential ──► Execute ──► Leak Scan ──► WASM
         Validator     (request)     Injector       Request     (response)
```

### Prompt Injection Defense

External content passes through multiple security layers:

- Pattern-based detection of injection attempts
- Content sanitisation and escaping
- Policy rules with severity levels (Block / Warn / Review / Sanitise)
- Tool output wrapping for safe LLM context injection

### Data Protection

- All data stored locally in your database (libSQL embedded by default)
- Secrets encrypted with AES-256-GCM
- No telemetry, analytics, or data sharing
- Full audit log of all tool executions

See [`docs/security.mdx`](docs/security.mdx) for the full security model.

---

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                          Channels                              │
│  ┌──────┐  ┌──────┐   ┌─────────────┐  ┌─────────────┐        │
│  │ TUI  │  │ HTTP │   │WASM Channels│  │ Web Gateway │        │
│  └──┬───┘  └──┬───┘   └──────┬──────┘  │ (SSE + WS)  │        │
│     │         │              │         └──────┬──────┘        │
│     └─────────┴──────────────┴────────────────┘               │
│                              │                                 │
│                    ┌─────────▼─────────┐                       │
│                    │    Agent Loop     │  Intent routing       │
│                    └────┬──────────┬───┘                       │
│                         │          │                           │
│              ┌──────────▼──┐  ┌────▼─────────────┐            │
│              │   Router    │  │  Mission Manager  │            │
│              │ (v2 engine) │  │ (cron/idle tasks) │            │
│              └──────┬──────┘  └────────┬──────────┘            │
│                     │                  │                       │
│       ┌─────────────▼──────────────────┘                       │
│       │                                                        │
│   ┌───▼──────────────────────────────────┐                     │
│   │            Engine v2                 │                     │
│   │  ┌──────────────┐ ┌───────────────┐  │                     │
│   │  │ Plan-first   │ │  Token Guard  │  │                     │
│   │  │  Execution   │ │  (8192 limit) │  │                     │
│   │  └──────┬───────┘ └───────┬───────┘  │                     │
│   │         │                 │          │                     │
│   │  ┌──────▼─────────────────▼───────┐  │                     │
│   │  │     Python Orchestrator        │  │                     │
│   │  │  (planning + tool dispatch)    │  │                     │
│   │  └────────────────────────────────┘  │                     │
│   └──────────────────┬───────────────────┘                     │
│                      │                                         │
│           ┌──────────▼──────────┐                              │
│           │    Tool Registry    │                              │
│           │  Built-in, MCP, WASM│                              │
│           └─────────────────────┘                              │
└────────────────────────────────────────────────────────────────┘
```

### Core Components

| Component | Purpose |
|-----------|---------|
| **Agent Loop** | Main message handling and thread coordination |
| **Router** | Classifies user intent and dispatches to engine |
| **Engine v2** | Plan-first execution with token budget enforcement |
| **Token Guard** | Hard-limits total prompt tokens; degrades gracefully |
| **Python Orchestrator** | LLM reasoning loop, tool calls, plan tracking |
| **Mission Manager** | Cron, idle, and event-triggered background tasks |
| **Tool Registry** | Built-in tools, MCP servers, WASM plugins |
| **Skill Selector** | Scores and injects relevant skill context within budget |
| **Memory Store** | Hybrid full-text + vector search with RRF |
| **Web Gateway** | Browser UI with chat, memory, routines, settings |

---

## Development

```bash
# Format code
cargo fmt

# Lint
cargo clippy --all --benches --tests --examples --all-features -- -D warnings

# Run unit and integration tests
cargo test --workspace

# Run with fast test runner (if nextest is installed)
cargo nextest run

# Python orchestrator syntax check
python3 -m py_compile crates/ironclaw_engine/orchestrator/default.py

# Run with debug logging
RUST_LOG=ironclaw=debug IRONCLAW_PROFILE=local cargo run
```

### Useful flags

```bash
# Enable trace-level logs for a specific module
RUST_LOG=ironclaw_engine=trace cargo run

# Override the token budget for testing
IRONCLAW_PROFILE=local cargo run   # 8192 tokens
```

---

## OpenClaw Heritage

IronClaw is a Rust reimplementation inspired by [OpenClaw](https://github.com/openclaw/openclaw). See [FEATURE_PARITY.md](FEATURE_PARITY.md) for the complete tracking matrix.

Key differences:

- **Rust vs TypeScript** — native performance, memory safety, single binary
- **WASM sandbox vs Docker** — lightweight, capability-based security
- **libSQL embedded vs SQLite** — zero-config local deployment
- **Engine v2** — plan-first execution with token budget enforcement, designed for local LLMs

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
