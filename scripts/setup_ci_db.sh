#!/usr/bin/env bash
set -euo pipefail

# Run after GitHub Actions has marked its fresh PostgreSQL service healthy.
# The service uses the workflow's postgres/password superuser credentials.
POSTGRES_CONTAINER="${1:?Pass the PostgreSQL service container ID}"
: "${GITHUB_ENV:?Run in GitHub Actions, or set GITHUB_ENV to a temporary file}"

# Match Cargo.lock, Dockerfile.migrations, and the query-checking documentation.
cargo install sqlx-cli --version 0.8.6 --locked \
    --no-default-features --features rustls,postgres

# Match scripts/init_db.sh's test connection budget. Revisit test-server cleanup
# if the suite outgrows this limit. max_connections requires a server restart.
docker exec -i "${POSTGRES_CONTAINER}" psql -X -U postgres -d postgres \
    -v ON_ERROR_STOP=1 -c 'ALTER SYSTEM SET max_connections = 1000;'
docker restart "${POSTGRES_CONTAINER}" > /dev/null
for attempt in {1..30}; do
    if docker exec "${POSTGRES_CONTAINER}" pg_isready -h 127.0.0.1 -U postgres -d postgres; then
        break
    fi
    if [[ "${attempt}" == 30 ]]; then
        echo >&2 "PostgreSQL did not become ready after increasing max_connections."
        exit 1
    fi
    sleep 2
done

# Use the service's client so the runner needs no PostgreSQL package installation.
docker exec -i "${POSTGRES_CONTAINER}" psql -X -U postgres -d postgres \
    -v ON_ERROR_STOP=1 <<'SQL'
CREATE USER app WITH PASSWORD 'secret' CREATEDB;
SQL

# POSTGRES_PORT allows this setup to be verified with an isolated local container.
export DATABASE_URL="postgres://app:secret@localhost:${POSTGRES_PORT:-5432}/alleycat"
sqlx database create --no-dotenv
sqlx migrate run --no-dotenv

# Subsequent test, coverage, and prepare steps must use the same migrated schema.
printf 'DATABASE_URL=%s\n' "${DATABASE_URL}" >> "${GITHUB_ENV}"
