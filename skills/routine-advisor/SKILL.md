---
name: routine-advisor
version: 0.1.0
description: Suggests relevant cron routines based on user context, goals, and observed patterns
activation:
  keywords:
    - every day
    - every morning
    - every week
    - routine
    - automate
    - remind me
    - check daily
    - monitor
    - recurring
    - schedule
    - habit
    - repetitive
  patterns:
    - "I (always|usually|often|regularly) (check|do|look at|review)"
    - "every (morning|evening|week|day|monday|friday)"
    - "I (wish|want) (I|it) (could|would) (automatically|auto)"
    - "I keep (forgetting|missing|having to)"
  tags:
    - automation
    - scheduling
    - personal-assistant
    - productivity
  max_context_tokens: 320
---

# Routine Advisor

Suggest recurring routines when user describes repetitive tasks.

Suggest when: user mentions repeating tasks, forgetting recurring work, or wanting automation. Not for one-time requests. Max 1 suggestion per conversation.

Be specific: "Want a daily 9am routine for a PR summary?"

After confirmation:
- Check `routine_list` first (avoid duplicates)
- `routine_create(trigger_type="cron", schedule="...", prompt="...")`
