---
title: Local Tools
description: Browser, calendar, notes, and file search — all running on your hardware
---

IronClaw ships with a `home` tool bundle containing four local tools. None of them require a cloud account, API key, or external service (except CalDAV, which requires a CalDAV-compatible server you control).

---

## Tool Bundle: `home`

The `home` bundle is enabled by default in the `local` profile. It contains:

| Tool | Registry entry | Tags |
|------|---------------|------|
| Browser (Playwright MCP) | `mcp-servers/playwright` | `default`, `local`, `browser` |
| CalDAV | `tools/caldav` | `opt-in`, `local`, `calendar` |
| Local Notes | `tools/local_notes` | `default`, `local`, `notes` |
| Local Search | `tools/local_search` | `default`, `local`, `search` |

---

## Browser Tool (Playwright MCP)

The browser tool uses [Playwright](https://playwright.dev/) via the MCP protocol to let IronClaw navigate websites, extract content, fill forms, and take screenshots.

### Prerequisites

- **Node.js 18+** — [nodejs.org](https://nodejs.org)
- **npx** — bundled with Node.js

### Installation

```bash
# Install Playwright globally
npm install -g @playwright/test

# Install the Chromium browser engine
npx playwright install chromium
```

Verify the installation:

```bash
npx playwright --version
```

IronClaw detects `npx` automatically on startup. If `node` or `npx` are missing, the browser capability shows as `NeedsSetup` in the status panel.

### Security

The browser tool enforces these security policies:

- **`file://` URLs are blocked** — the tool cannot read local files through the browser. Use the `local_search` tool for filesystem access instead.
- **Localhost access is allowed** — you can browse `http://localhost:3000` for local development.
- **HTTPS and HTTP URLs are allowed** — unrestricted web access.

If a request to browse a `file://` URL is made, the tool returns:

```
Error: file:// URLs are blocked for security. Use the local_search tool for filesystem access.
```

### Example usage

```
> Browse https://news.ycombinator.com and list the top 5 stories with their scores
> Take a screenshot of https://example.com
> Go to my router admin panel at http://192.168.1.1 and tell me the connected devices
```

### Configuring a different browser

By default, Playwright uses Chromium. To use Firefox or WebKit:

```bash
npx playwright install firefox
```

Then set the browser in `~/.ironclaw/config.toml`:

```toml
[tools.playwright]
browser = "firefox"   # chromium | firefox | webkit
```

---

## CalDAV (Calendar)

The CalDAV tool reads and writes calendar events and todos using the CalDAV protocol. Compatible with:

- **Nextcloud** (self-hosted)
- **Radicale** (self-hosted, lightweight)
- **Baikal** (self-hosted)
- **iCloud** (Apple's CalDAV endpoint)
- **Fastmail**
- **Google Calendar** (via the CalDAV interface — requires app password)
- Any RFC 4791-compliant CalDAV server

### Configuration

```env
CALDAV_URL=https://your-server/dav/calendars/username/
CALDAV_USERNAME=your-username
CALDAV_PASSWORD=your-password
```

Or in `~/.ironclaw/.env`:

```env
CALDAV_URL=https://nextcloud.example.com/remote.php/dav/calendars/alice/
CALDAV_USERNAME=alice
CALDAV_PASSWORD=hunter2
```

### iCloud CalDAV

iCloud uses a specific URL format:

```env
CALDAV_URL=https://caldav.icloud.com/
CALDAV_USERNAME=your-apple-id@icloud.com
CALDAV_PASSWORD=your-app-specific-password
```

> **App-specific password required:** Go to [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords → Generate password.

### Google Calendar via CalDAV

```env
CALDAV_URL=https://www.google.com/calendar/dav/your-email@gmail.com/events/
CALDAV_USERNAME=your-email@gmail.com
CALDAV_PASSWORD=your-app-password
```

> **App password required:** Go to your Google Account → Security → 2-Step Verification → App passwords.

### Available actions

| Action | Description |
|--------|-------------|
| `list_calendars` | List all calendars on the server |
| `list_events` | List events in a time range |
| `create_event` | Create a new calendar event |
| `update_event` | Update an existing event |
| `delete_event` | Delete an event |
| `list_todos` | List todos/tasks |
| `create_todo` | Create a new todo |
| `update_todo` | Update a todo |
| `delete_todo` | Delete a todo |

### Example usage

```
> What meetings do I have this week?
> Schedule a team standup every Monday at 9am
> What tasks are on my todo list?
> Mark the "review PR" task as done
```

---

## Local Notes

The notes tool provides a simple append-and-search interface to a plain-text file at `~/.ironclaw/notes.md`.

### Zero configuration

No setup required. The notes file is created automatically on first use.

### Available actions

| Action | Description |
|--------|-------------|
| `append_note` | Append a timestamped note to the file |
| `read_notes` | Read all notes |
| `search_notes` | Search notes by keyword |

### Storage location

```
~/.ironclaw/notes.md
```

You can open this file in any text editor. It is plain Markdown — readable, portable, and easy to back up.

### Example usage

```
> Remember that I need to renew the domain on June 15th
> What did I note about the API design?
> Show me all notes from this week
> Add a note: the production deploy went smoothly
```

---

## Local Search (Workspace File Search)

The local search tool searches files within the current workspace using pattern matching and content search.

### Zero configuration

No setup required. The tool searches the current workspace directory by default.

### Available actions

| Action | Parameters | Description |
|--------|-----------|-------------|
| `search_files` | `pattern`, `scope` | Search files by name or content |

### Scope control

By default, the tool is restricted to the current workspace. Global filesystem search is disabled for safety.

To allow searching beyond the workspace:

```toml
# ~/.ironclaw/config.toml
[local_search]
allow_global_scope = true
```

Or via the Settings UI: **Settings → Tools → Local Search → Allow whole-filesystem search**.

If a request for `scope: "global"` arrives when global scope is disabled:

```
Error: Global filesystem search is disabled. Enable it in Settings → Tools → Local Search.
```

### Example usage

```
> Find all Python files in this project that import requests
> Search for files modified in the last 7 days
> Find the config file — I think it's somewhere in the src directory
> Show me all TODO comments in the codebase
```

---

## Checking Tool Status

From the TUI or web UI, go to **Settings → Tools** to see which tools are:

- **Active** — installed and ready
- **NeedsSetup** — prerequisites missing (e.g. `npx` not found for the browser tool)
- **Disabled** — turned off manually

From the CLI:

```bash
ironclaw tools list
```

---

## Adding to the Home Bundle

The `home` bundle is defined in `registry/_bundles.json`:

```json
{
  "home": {
    "tools": ["browser", "caldav", "notes", "local-search"],
    "description": "Local tools for home use — no cloud accounts required"
  }
}
```

To add a custom tool to your bundle, install the tool registry file in `registry/tools/` or `registry/mcp-servers/` and update your config to include it.
