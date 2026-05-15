---
name: linear
version: "1.2.0"
description: Linear issue tracker API integration. Covers identity bootstrap, GraphQL for list/search/create/update.
activation:
  keywords:
    - "linear"
    - "linear issue"
    - "linear issues"
    - "linear ticket"
    - "linear backlog"
    - "my linear issues"
    - "linear.app"
  exclude_keywords:
    - "jira"
    - "asana"
    - "github issue"
  patterns:
    - "(?i)linear\\.(?:app|com)"
    - "(?i)\\blinear\\b.+(issue|ticket|task|backlog|board)"
    - "(?i)(create|show|list|close|update).+linear\\s+(issue|ticket)"
  tags:
    - "project-management"
    - "issue-tracking"
  max_context_tokens: 256
requires:
  env:
    - LINEAR_API_KEY
credentials:
  - name: linear_api_key
    provider: linear
    location:
      type: header
      name: Authorization
    hosts:
      - "api.linear.app"
    setup_instructions: "Create an API key at https://linear.app/settings/api"
---

# Linear API

GraphQL API at `https://api.linear.app/graphql`. Credentials auto-injected.

## Identity bootstrap
Cache `context/intel/linear-identity.md` (user_id, teams). If missing/stale, run:
`query { viewer { id name } teams(first: 50) { nodes { id key name } } }`
Update `stale_after` = today + 30 days.

## Common queries (POST with body={query, variables})
- List my issues: filter by `assignee: { id: { eq: "<cached_user_id>" } }`
- Get issue: `query($id: String!) { issue(id: $id) { id title description state { name } } }`
- Search: `query($term: String!) { issueSearch(query: $term, first: 10) { nodes { id identifier title } } }`
- Create: `mutation($input: IssueCreateInput!) { issueCreate(input: $input) { success issue { id url } } }` — teamId required

## Response format
`{"data": {...}}` on success, `{"errors": [...]}` on failure. Always check errors first.
- identifier = human-readable (ENG-42), id = UUID
