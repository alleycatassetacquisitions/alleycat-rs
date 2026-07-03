#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

if [[ ! -f ".env" ]]; then
    echo >&2 "Missing .env."
    echo >&2 "Create it with: cp .env.example .env"
    exit 1
fi

if ! command -v bunyan >/dev/null 2>&1; then
    echo >&2 "bunyan is not installed; showing raw app logs."
    echo >&2 "Install it with: cargo install bunyan"
    exec docker compose logs -f app
fi

docker compose logs --no-color --no-log-prefix -f app | bunyan
