---
name: security-review
version: 0.1.0
description: Security audit for code changes — injection, auth, data exposure, crypto, secrets.
activation:
  keywords:
    - security review
    - security audit
    - vulnerability
    - OWASP
    - injection
    - auth security
    - secrets exposure
    - security check
  patterns:
    - "(?i)(security|vulnerability) (review|audit|check|scan)"
    - "(?i)check (for )?(vulnerabilities|security|injection)"
    - "(?i)is (this|it) (secure|safe)"
  tags:
    - developer
    - security
    - review
  max_context_tokens: 320
---

# Security Review

Check code for vulnerabilities:
1. Injection: trace user input to DB/shell/template; use parameterized queries
2. Auth: session tokens secure, IDOR checks, API keys not hardcoded or logged
3. Data exposure: no stack traces or PII in errors; CORS restrictive
4. Crypto: AES-256-GCM, crypto-secure RNG, TLS enforced
5. Secrets: grep for hardcoded tokens; verify .env is gitignored

Output: findings with P1/P2/P3 severity, file:line, risk, concrete fix. Auto-fix obvious issues.
