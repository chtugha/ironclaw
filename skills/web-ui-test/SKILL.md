---
name: web-ui-test
version: 0.1.0
description: Test the IronClaw web UI using Playwright browser automation.
activation:
  keywords:
    - test web ui
    - test the ui
    - browser test
    - test skills tab
    - test chat
    - web gateway test
  patterns:
    - "test.*web.*ui"
    - "test.*browser"
  max_context_tokens: 256
---

# Web UI Testing

Test the IronClaw web UI via Playwright browser automation.

## Setup
Run: `CLI_ENABLED=false GATEWAY_AUTH_TOKEN=<token> cargo run`
Open: `http://127.0.0.1:3000/?token=<token>`

## Test checklist
1. Verify "Connected" indicator and all tabs: Chat, Memory, Jobs, Routines, Extensions, Skills
2. Send test message, verify LLM responds
3. Skills tab: verify list loads, search works
4. Install skill by URL, verify success, remove it
5. Other tabs: Memory, Jobs, Routines, Extensions (smoke test)
