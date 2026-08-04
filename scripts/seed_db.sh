#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SEED_FILE="${SCRIPT_DIR}/seed_db.sql"

if [[ -n "${DATABASE_URL:-}" ]]; then
    if ! command -v psql >/dev/null 2>&1; then
        echo >&2 "Error: psql is required when seeding through DATABASE_URL."
        exit 1
    fi

    psql "${DATABASE_URL}" --no-psqlrc --set ON_ERROR_STOP=1 --file "${SEED_FILE}"
else
    cd "${PROJECT_ROOT}"

    if [[ ! -f ".env" ]]; then
        echo >&2 "Missing .env."
        echo >&2 "Create it with: cp .env.example .env"
        exit 1
    fi

    if [[ "$(docker compose ps --status running --services postgres)" != "postgres" ]]; then
        echo >&2 "The local Postgres container is not running."
        echo >&2 "Start it with: scripts/start_local_instance.sh"
        exit 1
    fi

    docker compose exec -T postgres sh -c \
        'PGPASSWORD="$POSTGRES_PASSWORD" psql -h localhost -U "$POSTGRES_USER" "$POSTGRES_DB" --no-psqlrc --set ON_ERROR_STOP=1' \
        < "${SEED_FILE}"
fi

echo "Seeded sample players and device logs."
