-- Seed the default prompt token ceiling for the agent settings.
--
-- Inserts a default row for `agent.max_prompt_tokens` so that existing
-- installations start with the 8192-token ceiling without requiring a
-- manual settings update. Uses INSERT OR IGNORE so re-running the
-- migration against an already-seeded database is a no-op.

INSERT OR IGNORE INTO settings (user_id, key, value, updated_at)
VALUES ('system', 'agent.max_prompt_tokens', '8192', CURRENT_TIMESTAMP);
