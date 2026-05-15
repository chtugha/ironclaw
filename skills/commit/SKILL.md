---
name: commit
version: "1.0.0"
description: Generate git commit messages from staged changes
activation:
  keywords:
    - "commit"
    - "git commit"
  patterns:
    - "(?i)(create|make|write|generate)\\s.*commit"
    - "(?i)commit\\s.*(message|changes|staged)"
  tags:
    - "git"
    - "version-control"
  max_context_tokens: 128
---

# Git Commit

1. `git status`, `git diff --cached`, `git log --oneline -5`
2. Draft message: concise, match repo style, focus on why
3. Format: `<type>: <description>` (fix|feat|refactor|test|docs|chore)
4. Warn on staged secrets (.env, credentials files)
5. Show message, ask confirmation, then `git commit`
6. Stage files individually, never `git add -A` or `.`
