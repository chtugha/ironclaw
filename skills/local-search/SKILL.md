---
name: local-search
version: "1.0.0"
description: Search files in the workspace using regex patterns
activation:
  keywords:
    - "search"
    - "find"
    - "grep"
    - "locate"
    - "look for"
    - "where is"
  patterns:
    - "(?i)(find|search|grep|locate)\\s.*(file|code|text|function)"
    - "(?i)where\\s.*(defined|used|called)"
  tags:
    - "search"
    - "local"
  max_context_tokens: 256
---

# Local File Search

Use the `local_search` tool with action `search_files`.

Key params: `pattern` (regex, required), `path` (directory), `glob` (file filter), `output_mode` (`files_with_matches` default, `content`, `count`).

Default `scope` is `workspace` — safe and sandboxed. Do not request `scope: global` unless the user explicitly asks for a whole-filesystem search.
