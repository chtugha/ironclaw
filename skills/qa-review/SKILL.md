---
name: qa-review
version: 0.1.0
description: QA review for code changes — test coverage analysis, edge case identification, test plan generation.
activation:
  keywords:
    - QA review
    - test coverage
    - test plan
    - quality check
    - edge cases
    - regression test
    - missing tests
    - testing review
  patterns:
    - "(?i)(QA|quality|test|testing) (review|check|audit|plan)"
    - "(?i)(check|review|improve) (test )?coverage"
    - "(?i)what (edge cases|tests) am I missing"
    - "(?i)generate (a )?test plan"
  tags:
    - developer
    - testing
    - review
  max_context_tokens: 128
---

# QA Review

Review code for test coverage and edge cases.

1. Find untested paths: error handlers, boundaries, null inputs
2. Flag missing edge cases: empty, zero, max, concurrent, external failure
3. Identify regression risks from changed behavior
4. Generate test plan if asked: unit, integration, regression, manual steps

Output: Coverage gaps, missing edge cases, regression risks, health score (0-100).
