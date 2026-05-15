---
name: caldav
version: "1.0.0"
description: Manage calendar events and todos via CalDAV
activation:
  keywords:
    - "calendar"
    - "event"
    - "meeting"
    - "schedule"
    - "appointment"
    - "todo"
    - "reminder"
  patterns:
    - "(?i)(add|create|new|schedule)\\s.*(event|meeting|appointment)"
    - "(?i)(list|show|what).*(calendar|events|meetings)"
    - "(?i)(delete|remove|cancel).*(event|meeting)"
  tags:
    - "calendar"
    - "local"
  max_context_tokens: 256
---

# CalDAV Calendar

Use the `caldav` tool to manage calendar events and tasks.

## Key actions

- `list_calendars` — show available calendars
- `list_events` — list events in a date range (params: `calendar_id`, `start`, `end` as ISO-8601)
- `create_event` — add an event (params: `calendar_id`, `summary`, `start`, `end`, optional `description`)
- `update_event` — edit an existing event (`event_id` required)
- `delete_event` — remove an event (`event_id` required)
- `list_todos`, `create_todo`, `update_todo`, `delete_todo` — same pattern for tasks

## Rules

1. Always call `list_calendars` first if no `calendar_id` is known.
2. Confirm with the user before deleting events.
3. Use ISO-8601 timestamps (e.g. `2026-05-15T10:00:00`).
