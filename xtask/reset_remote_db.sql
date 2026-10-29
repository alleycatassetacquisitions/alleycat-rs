BEGIN;
SET LOCAL search_path TO public;
-- Remove application tables and SQLx's migration ledger. CASCADE removes
-- dependent constraints and indexes.
DROP TABLE IF EXISTS
    pdn_inspection_checks,
    device_inspections,
    devices,
    player_affiliations,
    affiliations,
    affiliation_categories,
    quickdraw_matches,
    event_scoring_periods,
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
    event_uuid_range,
    quickdraw_scoring_status,
    player_role,
    player_team,
    player_mode
CASCADE;
-- btree_gist is shared infrastructure, not application data. Preserve it and
-- any unrelated objects that depend on it, including on databases that applied
-- the original scoring-period migration.
COMMIT;
