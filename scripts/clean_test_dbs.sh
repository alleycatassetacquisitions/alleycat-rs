#!/usr/bin/env bash
set -euo pipefail

SUPERUSER="${SUPERUSER:=postgres}"

docker exec -i postgres psql -U "${SUPERUSER}" -d postgres -v ON_ERROR_STOP=1 <<'SQL'
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE datname LIKE 'alleycat_test_%'
  AND pid <> pg_backend_pid();

SELECT format('DROP DATABASE IF EXISTS %I;', datname)
FROM pg_database
WHERE datistemplate = false
  AND datname LIKE 'alleycat_test_%'
\gexec
SQL
