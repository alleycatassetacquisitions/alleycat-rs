BEGIN;
SET LOCAL search_path TO public;
-- Remove application tables and SQLx's migration ledger. CASCADE removes
-- dependent constraints and indexes.
DROP TABLE IF EXISTS
    device_logs,
    player_roles,
    players,
    teams,
    app_state,
    events,
    reserved_pdn_codes,
    available_pdn_codes,
    _sqlx_migrations
CASCADE;

-- Remove application-defined PostgreSQL types after their tables are gone.
-- Include the legacy name for databases that have not applied the team rename.
DROP TYPE IF EXISTS
    player_role,
    player_team,
    player_mode
CASCADE;
COMMIT;
