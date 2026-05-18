---
name: product-prioritization
version: 0.1.0
description: Product strategy and feature prioritization — score features by evidence, effort, and strategic alignment.
activation:
  keywords:
    - prioritize
    - what to build
    - roadmap
    - feature priority
    - product strategy
    - user demand
    - worth building
    - should we build
  patterns:
    - "(?i)(prioritize|rank|score) (features|tasks|backlog|ideas)"
    - "(?i)what should (we|I) (build|work on|focus on) (next|first)"
    - "(?i)is (this|it) worth (building|doing)"
    - "(?i)(product|roadmap|strategy) (review|planning)"
  tags:
    - product
    - strategy
    - prioritization
  max_context_tokens: 448
---

# Product Prioritization

Evidence-based feature prioritization. Challenge assumptions, never validate opinions.

## Forcing questions (ask before scoring)
1. Who specifically needs this? (name a real user, not "everyone")
2. What evidence? (tickets, churn, interviews — not "I think")
3. What happens if we don't build it?
4. Smallest version that delivers value?

## Scoring: Priority = (Demand×3 + Impact×2 + Alignment×1) / Effort
Each dimension 1-10. Demand weighted highest (hardest to fake). Effort uses dual estimate (human vs AI time).

## Modes
- **Score one**: run forcing questions then score table
- **Rank backlog**: read commitments/open/, parked-ideas/, tech-debt/ — quick score each, present ranked table with build/defer/kill/investigate labels
- **Analyze feedback**: extract themes, cluster, score by frequency × severity

## Anti-patterns
Reject: building for yourself, competitor copying, sunk cost, "just one more thing", one user = demand.
