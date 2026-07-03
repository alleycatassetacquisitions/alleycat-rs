#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BACKUP_DIR="${PROJECT_ROOT}/backups/logs"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
BACKUP_FILE="${BACKUP_DIR}/app-${TIMESTAMP}.jsonl"

cd "${PROJECT_ROOT}"
umask 077
mkdir -p "${BACKUP_DIR}"

docker compose logs --no-color --no-log-prefix app > "${BACKUP_FILE}"

echo "Backed up app logs to ${BACKUP_FILE}"
