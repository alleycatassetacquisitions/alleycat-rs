# Script reference

The twelve executable helpers target different databases. Identify the target
before running one.

| Target | How to recognize it | Scripts |
| --- | --- | --- |
| Production-like local Compose | `docker compose`, `.env`, named volume | `start_local_instance`, `view_logs`, both backups, `delete_local_instance` |
| Compose or explicit URL | `DATABASE_URL` selects the target when set | `seed_db` |
| Native development/test DB | Standalone container literally named `postgres` | `init_db`, `kill_db`, `reset_db`, `prepare_sqlx`, `clean_test_dbs` |
| Disposable DigitalOcean DB | Explicit remote `DATABASE_URL` | `reset_remote_db` only |

The Compose and native-development databases are independent. The Compose
PostgreSQL port is not exposed to the host. Native helpers do not understand
the Compose service or its volume.

## Safety at a glance

| Script | Safety | Main effect |
| --- | --- | --- |
| `start_local_instance.sh` | Mutating, preserves data | Starts/recreates the Compose stack and applies pending migrations |
| `seed_db.sh` | Mutates rows | Upserts repeatable sample data into the selected database |
| `view_logs.sh` | Read-only | Follows Compose API logs |
| `backup_logs.sh` | Read-only toward services | Writes retained API logs under `backups/` |
| `backup_db.sh` | Read-only toward database | Writes a SQL dump under `backups/` |
| `delete_local_instance.sh` | **Destructive local, prompted** | Backs up, then deletes the Compose database volume and containers |
| `init_db.sh` | Mutating | Creates and migrates the standalone native database |
| `kill_db.sh` | **Destructive local, no prompt** | Force-removes the standalone `postgres` container and its data |
| `reset_db.sh` | **Destructive local, no prompt** | Removes and recreates the standalone database |
| `prepare_sqlx.sh` | **Destructive local, no prompt** | Resets the standalone database and regenerates `.sqlx/` |
| `clean_test_dbs.sh` | **Destructive; intended test scope has a wildcard caveat** | Drops databases matched by its current SQL pattern |
| `reset_remote_db.sh` | **Destructive remote, prompted** | Permanently wipes a disposable DigitalOcean DB and replays migrations |

## Compose stack scripts

### `scripts/start_local_instance.sh`

- Requires a `.env` containing `POSTGRES_USER`, `POSTGRES_PASSWORD`, and
  `POSTGRES_DB`.
- Removes an old one-shot `migrate` service container so migrations run on
  every start.
- Runs `docker compose up -d`; existing PostgreSQL data is preserved.
- Does not rebuild stale images or wait for API/UI readiness. See the
  [Compose workflow](operations.md#production-like-local-stack).

### `scripts/view_logs.sh`

Follows API container logs. It requires `.env` and the Compose stack. If a
`bunyan` executable is on `PATH`, it formats the logs; otherwise it shows raw
`docker compose logs -f app` output. It does not show UI logs.

### `scripts/backup_logs.sh`

Copies all API logs still retained by Docker to:

```text
backups/logs/app-<UTC timestamp>.jsonl
```

It captures only the `app` service, not web, database, or migration logs.
Docker log rotation means older output may already be gone. The script uses
`umask 077`, and `backups/` is gitignored.

Although the script does not inspect `.env`, Compose may need it to interpolate
the project file.

### `scripts/backup_db.sh`

Runs `pg_dump` inside the Compose PostgreSQL container and writes plain SQL to:

```text
backups/db/alleycat-<UTC timestamp>.sql
```

It requires `.env` and a running Compose database. Files are private by
default and gitignored. There is no restore or verification helper. If
`pg_dump` fails, shell redirection may leave an empty or partial timestamped
file; confirm a backup before relying on it.

### `scripts/delete_local_instance.sh`

Use this only when the Compose stack and its data should be removed.

1. It explains the scope and requires the exact phrase
   `delete local instance`.
2. It runs `backup_logs.sh` and `backup_db.sh`.
3. Only if both succeed, it runs
   `docker compose down --volumes --remove-orphans`.

It deletes the Compose containers, network, orphaned containers, and database
volume. It leaves images, `.env`, and backup files. Recovery requires manually
restoring the generated SQL dump. PostgreSQL must be running when the script
starts because `backup_db.sh` uses `docker compose exec`; if it is stopped,
resume the stack before deletion.

## Cross-target seeding

### `scripts/seed_db.sh`

`scripts/seed_db.sql` transactionally upserts five players and 12 device logs.
The target depends on the environment:

1. If `DATABASE_URL` is non-empty, the script requires host `psql` and applies
   the seed to that URL.
2. Otherwise, it uses `.env` and the running Compose `postgres` service.

The URL mode has no local/remote guard or confirmation. An inherited
`DATABASE_URL` can select a shared database unintentionally. Inspect it before
seeding, or remove it to target Compose:

```bash
unset DATABASE_URL
scripts/seed_db.sh
```

The seed reuses IDs and unique keys but updates rows and timestamps. It is not
read-only.

## Native development and test database

### `scripts/init_db.sh`

Use it once to create the standalone database used by host-native Rust and the
API tests.

The script:

1. verifies that `sqlx` is installed;
2. starts an unpinned `postgres` Docker image as a container named `postgres`,
   publishing its port to the host;
3. waits for PostgreSQL health;
4. creates an application user and grants it `CREATEDB`;
5. creates the application database and applies all migrations.

Defaults and overrides are:

| Variable | Default | Meaning |
| --- | --- | --- |
| `POSTGRES_PORT` | `5432` | Published host port |
| `SUPERUSER` | `postgres` | PostgreSQL administration user |
| `SUPERUSER_PWD` | `password` | Administration password |
| `APP_USER` | `app` | Database owner/application user |
| `APP_USER_PWD` | `secret` | Application password |
| `APP_DB_NAME` | `alleycat` | Application database |
| `SKIP_DOCKER` | unset | Any non-empty value skips container and role creation, as CI does |

Run from the repository root because SQLx locates `migrations/` from the
current directory. The script does not read `.env` and is not idempotent: an
existing container or role makes it fail. Start a stopped existing container
with `docker start postgres` instead.

It enables shell tracing, which prints commands containing credentials. Use
only throwaway local credentials. Native `cargo run` connects as
`postgres/password` by default, while SQLx preparation uses `app/secret`; both
roles exist after this setup.

### `scripts/kill_db.sh`

Runs `docker rm -f -v postgres`. It force-removes the standalone container,
including its database data, without confirmation or backup. It can also remove
an unrelated container if that container happens to be named `postgres`, so
resolve the target first:

```bash
docker ps -a --filter name=^/postgres$
```

The command fails when the container does not exist. To pause while preserving
data, use `docker stop postgres` instead.

### `scripts/reset_db.sh`

Calls `kill_db.sh` and then `init_db.sh`. It irreversibly replaces all data in
the standalone native database, without a prompt or backup. Because the kill
step fails when `postgres` is absent, use `init_db.sh` for true first-time
setup.

### `scripts/prepare_sqlx.sh`

Regenerates the checked-in compile-time query metadata under `.sqlx/`:

1. calls `reset_db.sh`, destroying and recreating the standalone database;
2. constructs its `DATABASE_URL` from the `APP_*`/port values;
3. runs `cargo sqlx prepare --workspace -- --all-targets`.

Run it from the repository root after a migration or checked SQL query changes.
It is not general test setup and should not be run when unbacked-up native data
matters. Docker release builds use `SQLX_OFFLINE=true`, so the regenerated
metadata normally belongs in the same commit as the schema/query change.

### `scripts/clean_test_dbs.sh`

API tests create a new migrated database for each test application and do not
drop it afterward. This script connects to the standalone container,
terminates connections, and drops databases matched by SQL
`LIKE 'alleycat_test_%'`.

It is intended to target the literal prefix `alleycat_test_`, but its SQL does
not escape either underscore. In a `LIKE` pattern, `_` matches any one
character, so other database names can match too. Resolve the candidates before
running it; tightening this pattern is a recommended script fix.

It accepts `SUPERUSER` (default `postgres`) and does not operate on the Compose
container.

## Remote disposable database

### `scripts/reset_remote_db.sh`

This is an emergency/development reset for a **disposable** DigitalOcean
database. It is never part of routine tests, deployment, or forward migration.

It requires:

- an explicit `DATABASE_URL` containing `ondigitalocean.com`;
- host `psql` and `sqlx` executables;
- network access and permission to modify the database; and
- either typing the parsed database name at the prompt or setting
  `CONFIRM_REMOTE_DB_RESET` to that exact name for intentional automation.

It explicitly drops `device_logs`, `player_roles`, `players`,
`available_pdn_codes`, SQLx's `_sqlx_migrations` ledger, and the `player_role`
and `player_mode` types. It then runs all migrations from scratch and prints
their status.

The connection-string substring guard and confirmation reduce mistakes; they
do not parse or prove the hostname, nor prove that the selected DigitalOcean
database is disposable. Resolve and verify the target yourself. Never run this
script merely to check a migration.

Whenever a migration adds, removes, or renames a table, view, sequence,
routine, or custom type, update this explicit reset inventory in the same
change. Do not replace it with a broad drop of DigitalOcean's provider-owned
`public` schema.

## Related commands

Cargo, npm, build, and CI commands are covered by the
[developer guide](development.md#tests-and-checks) and
[architecture code map](architecture.md#code-map).
