#!/usr/bin/env bash
# Reset a disposable DigitalOcean database and rebuild it from the migrations
# in this checkout. This permanently deletes all application data.
set -euo pipefail

# Require an explicit remote connection instead of falling back to `.env`.
: "${DATABASE_URL:?Set DATABASE_URL to the remote PostgreSQL connection string.}"

# Guard against accidentally running this destructive script on another host.
if [[ "${DATABASE_URL}" != *"ondigitalocean.com"* ]]; then
    echo >&2 "Refusing to reset a database outside DigitalOcean."
    exit 1
fi

# Extract the database name from the URL for the confirmation prompt.
DATABASE_NAME="${DATABASE_URL%%\?*}"
DATABASE_NAME="${DATABASE_NAME##*/}"

# Allow interactive confirmation by default. Automation can opt in by setting
# CONFIRM_REMOTE_DB_RESET to the exact database name.
if [[ "${CONFIRM_REMOTE_DB_RESET:-}" != "${DATABASE_NAME}" ]]; then
    echo >&2 "This will permanently delete all data in '${DATABASE_NAME}'."
    read -r -p "Type '${DATABASE_NAME}' to continue: " confirmation
    if [[ "${confirmation}" != "${DATABASE_NAME}" ]]; then
        echo >&2 "Reset cancelled."
        exit 1
    fi
fi

# DigitalOcean owns the public schema, so the application user cannot drop and
# recreate it. Drop the objects created by migrations explicitly instead.
# Keep this inventory synchronized with the files in migrations/.
psql "${DATABASE_URL}" --set ON_ERROR_STOP=1 <<'SQL'
-- Remove application tables and SQLx's migration ledger. CASCADE removes
-- dependent constraints and indexes.
DROP TABLE IF EXISTS
    device_logs,
    player_roles,
    players,
    available_pdn_codes,
    _sqlx_migrations
CASCADE;

-- Remove application-defined PostgreSQL types after their tables are gone.
DROP TYPE IF EXISTS
    player_role,
    player_mode
CASCADE;
SQL

# With the schema empty, apply the current migration history from scratch and
# print the resulting migration status.
sqlx migrate run --no-dotenv
sqlx migrate info --no-dotenv
