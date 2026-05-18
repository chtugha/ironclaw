---
name: coding
version: "1.0.0"
description: Best practices for code editing, search, and file operations
activation:
  keywords:
    - "code"
    - "edit"
    - "fix"
    - "implement"
    - "refactor"
    - "bug"
    - "function"
    - "class"
    - "file"
    - "module"
    - "compile"
    - "build"
    - "error"
    - "change"
    - "rename"
    - "delete"
    - "add"
    - "update"
  exclude_keywords:
    - "memory"
    - "routine"
    - "schedule"
  patterns:
    - "(?i)(add|remove|update|modify|create|delete|rename|move)\\s.*(file|function|class|method)"
    - "(?i)(fix|debug|investigate|trace|find)\\s.*(bug|error|issue|crash|fail)"
  tags:
    - "development"
    - "coding"
  max_context_tokens: 384
---

# Coding Best Practices

## Tool Usage
- `apply_patch` over `write_file` for existing files
- `read_file` before editing — always
- `glob` for file discovery, `grep` for content search, `list_dir` for directories

## Change Discipline
- Minimal changes — don't add features beyond what was asked
- No unnecessary comments, docstrings, or annotations
- One change at a time; fix the pattern, not just the instance (use `grep` to find all occurrences)
- Preserve existing code style, naming, indentation

## Quality
- Don't introduce: command injection, XSS, SQL injection, path traversal
- Test after changes if test infrastructure exists
- No error handling for impossible scenarios — trust internal guarantees
