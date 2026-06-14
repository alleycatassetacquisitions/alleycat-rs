#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DB_PORT="${POSTGRES_PORT:=5432}"
APP_USER="${APP_USER:=app}"
APP_USER_PWD="${APP_USER_PWD:=secret}"
APP_DB_NAME="${APP_DB_NAME:=alleycat}"

"${SCRIPT_DIR}/reset_db.sh"

export DATABASE_URL="postgres://${APP_USER}:${APP_USER_PWD}@localhost:${DB_PORT}/${APP_DB_NAME}"
cargo sqlx prepare --workspace -- --all-targets
