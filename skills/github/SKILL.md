---
name: github
version: "1.0.0"
description: GitHub API integration via HTTP tool with automatic credential injection
activation:
  keywords:
    - "github"
    - "issues"
    - "pull request"
    - "repository"
  exclude_keywords:
    - "gitlab"
    - "bitbucket"
  patterns:
    - "(?i)(list|show|get|fetch|open|close|create|merge)\\s.*(issue|PR|pull request|repo)"
    - "(?i)github\\.com"
  tags:
    - "git"
    - "code-review"
    - "devops"
  max_context_tokens: 512
requires:
  env:
    - GITHUB_TOKEN
credentials:
  - name: github_token
    provider: github
    location:
      type: bearer
    hosts:
      - "api.github.com"
    oauth:
      authorization_url: "https://github.com/login/oauth/authorize"
      token_url: "https://github.com/login/oauth/access_token"
      scopes:
        - "repo"
        - "read:org"
      refresh:
        strategy: reauthorize_only
    setup_instructions: "Create a personal access token at https://github.com/settings/tokens"
---

# GitHub API

Use `http` tool against `https://api.github.com`. Credentials auto-injected — never add Authorization headers manually.

## Common patterns
- Issues: `GET /repos/{owner}/{repo}/issues?state=open&per_page=30`
- Create issue: `POST /repos/{owner}/{repo}/issues` body={title, body, labels}
- PRs: `GET /repos/{owner}/{repo}/pulls?state=open`
- Create PR: `POST /repos/{owner}/{repo}/pulls` body={title, body, head, base, draft: true}
- PR diff: `GET /repos/.../pulls/{n}` with `Accept: application/vnd.github.v3.diff`
- My PRs: `GET /search/issues?q=is:pr+author:%40me+sort:updated-desc`

## Response format
`{"status": 200, "headers": {}, "body": <parsed JSON or string>}` — body is already parsed, don't call `json.loads()`. Fail fast on non-2xx.

## Rules
- Always set `draft: true` on new PRs unless user says "ready for review"
- URL-encode `@` as `%40`, spaces as `+` in query strings
- Max `per_page=100`; check `Link` header for pagination
