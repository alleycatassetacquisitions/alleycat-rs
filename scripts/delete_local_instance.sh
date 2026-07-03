#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

echo >&2 "This deletes the local Docker containers, network, and Postgres data volume."
echo >&2 "The script will back up app logs and the database before deleting anything."
read -r -p "Type 'delete local instance' to continue: " confirmation

if [[ "${confirmation}" != "delete local instance" ]]; then
    echo >&2 "Canceled."
    exit 1
fi

"${SCRIPT_DIR}/backup_logs.sh"
"${SCRIPT_DIR}/backup_db.sh"

docker compose down --volumes --remove-orphans
