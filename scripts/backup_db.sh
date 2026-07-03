#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BACKUP_DIR="${PROJECT_ROOT}/backups/db"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
BACKUP_FILE="${BACKUP_DIR}/alleycat-${TIMESTAMP}.sql"

cd "${PROJECT_ROOT}"

if [[ ! -f ".env" ]]; then
    echo >&2 "Missing .env."
    echo >&2 "Create it with: cp .env.example .env"
    exit 1
fi

umask 077
mkdir -p "${BACKUP_DIR}"

docker compose exec -T postgres sh -c \
    'PGPASSWORD="$POSTGRES_PASSWORD" pg_dump -h localhost -U "$POSTGRES_USER" "$POSTGRES_DB"' \
    > "${BACKUP_FILE}"

echo "Backed up database to ${BACKUP_FILE}"
