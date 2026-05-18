---
name: local-test
version: 0.1.0
description: Build, run, and test IronClaw locally using Docker containers and Playwright browser automation.
activation:
  keywords:
    - test locally
    - local test
    - docker test
    - test my changes
    - test in docker
    - test web gateway
    - spin up test
    - test container
  patterns:
    - "test.*local"
    - "docker.*test"
    - "spin.*up.*test"
  max_context_tokens: 384
---

# Local Testing

Build and run IronClaw locally with Docker, then test via Playwright browser.

## Quick start
```bash
docker build --platform linux/amd64 -f Dockerfile.test -t ironclaw-test .
docker run --rm -p 3003:3003 \
  -e ONBOARD_COMPLETED=true -e CLI_ENABLED=false \
  -e LLM_BACKEND=ollama -e OLLAMA_BASE_URL=http://host.docker.internal:11434 \
  ironclaw-test
```
Open: `http://localhost:3003/?token=test`

## Key env vars
- `ONBOARD_COMPLETED=true` — skip onboarding wizard
- `CLI_ENABLED=false` — disable TUI (prevents EOF shutdown)
- `GATEWAY_AUTH_TOKEN` — auth token (default: `test`)

## Browser testing
Use the `playwright` MCP tool to automate browser testing:
- Navigate to `http://localhost:3003/?token=test`
- Verify "Connected" indicator and all tabs visible
- Test chat, skills, memory, routines tabs

## Troubleshooting
- Container exits: missing `ONBOARD_COMPLETED=true` or `CLI_ENABLED=false`
- Port in use: change host port with `-p 3005:3003`
