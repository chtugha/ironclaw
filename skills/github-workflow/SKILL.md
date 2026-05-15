---
name: github-workflow
version: 0.1.0
description: Install and operate a full GitHub issue-to-merge workflow for any repository using event-driven and cron missions.
activation:
  keywords:
    - github workflow
    - CI automation
    - PR automation
    - automate repo
    - workflow setup
    - github automation
  patterns:
    - "(?i)(set ?up|install|enable).*(workflow|automation) for .+/.+"
    - "(?i)automate (this |my )?(repo|repository|PRs|CI)"
    - "(?i)github (workflow|automation|pipeline)"
  tags:
    - developer
    - github
    - automation
    - workflow
  max_context_tokens: 256
requires:
  env:
    - GITHUB_TOKEN
  skills:
    - github
---

# GitHub Workflow Automation

Install issue-to-merge automation for a GitHub repository.

## Setup
Read from `projects/<owner>-<repo>/project.md` or ask: `repository`, `maintainers`, `main_branch`, `staging_branch`.

1. Open `references/workflow-routines.md`
2. Replace placeholders ({{repository}}, {{slug}}, {{maintainers}}, branches)
3. Check `mission_list`; update rather than duplicate
4. `mission_create` for each template
5. Update project file: `workflow_installed: true`

## Missions created
- `wf-issue-plan`: on issue.opened — plan
- `wf-pr-monitor`: on PR events — feedback/refresh
- `wf-ci-fix`: on CI failure — fix and push
- `wf-learning`: on merge — extract lessons
