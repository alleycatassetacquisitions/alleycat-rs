#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

if [[ ! -f ".env" ]]; then
    echo >&2 "Missing .env."
    echo >&2 "Create it with: cp .env.example .env"
    echo >&2 "Then set POSTGRES_PASSWORD before starting the local instance."
    exit 1
fi

docker compose rm -f migrate >/dev/null 2>&1 || true
docker compose up -d

echo "Local instance is starting."
echo "Health check: curl -i http://localhost:8000/health_check"
echo "API logs: docker compose logs --no-log-prefix -f app | bunyan"
