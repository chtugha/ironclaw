---
name: code-review
version: "2.0.0"
description: Review local code changes for bugs, security, missing tests, and undocumented assumptions.
activation:
  keywords:
    - "review"
    - "code review"
    - "review changes"
  patterns:
    - "(?i)review\\s.*(code|changes|diff|commit)"
    - "(?i)(check|look at|inspect)\\s.*(changes|diff|code)"
  tags:
    - "code-review"
    - "quality"
    - "security"
  max_context_tokens: 256
---

# Code Review

1. Run `git diff`, `git diff --cached`, or `git diff HEAD~1`
2. Read each changed file in full for context
3. Check: correctness, edge cases, security (injection/auth/secrets), test coverage, docs
4. Report findings with file:line, severity (Critical/High/Medium/Low), and suggested fix
5. Be specific — cite exact lines, distinguish "is a bug" from "could be if X"
