---
name: project-setup
version: 0.1.0
description: Add a GitHub repository as a tracked project — creates workspace entity, installs workflow automation missions.
activation:
  keywords:
    - add repo
    - add repository
    - add project
    - track repo
    - setup repo
    - project setup
    - setup workflow
  patterns:
    - "(?i)(add|track|setup|install|enable) (repo|repository|project)\\s"
    - "(?i)add .+/.+ to my workflow"
    - "(?i)set ?up (workflow|automation) for .+/.+"
  tags:
    - developer
    - github
    - setup
  max_context_tokens: 256
requires:
  env:
    - GITHUB_TOKEN
  skills:
    - github
    - github-workflow
---

# Project Setup

Add a GitHub repo as a tracked project with workflow automation.

1. Extract `owner/repo`, validate via `http(GET, "https://api.github.com/repos/{owner}/{repo}")`
2. Ask: maintainers? staging branch? AI bot authors?
3. Write `projects/<owner>-<repo>/project.md` (repo metadata, workflow_installed: false)
4. Write `projects/<owner>-<repo>/notes.md`
5. Install workflow missions from `github-workflow` skill (open `references/workflow-routines.md`, replace placeholders, call `mission_create` for each)
6. Update project file: `workflow_installed: true`, list missions
7. Confirm: project path, missions installed, branches tracked

Requires GitHub skill authenticated.
