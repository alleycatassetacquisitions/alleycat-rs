# Alleycat server

Alleycat is a Rust HTTP API backed by PostgreSQL. This guide runs the API and
database together on one machine with Docker Compose.

> This deployment publishes an unauthenticated API on port `8000`. Use it only
> on a trusted network; do not expose it directly to the internet.

## Requirements

- Git
- [Docker with Docker Compose](https://docs.docker.com/get-started/get-docker/)
- [Rustup](https://rustup.rs/), which provides Cargo

You do not need Rust experience to deploy Alleycat. Rust is required to run the
repository's task interface. The repository selects Rust `1.95.0`
automatically; the first Cargo command may download that toolchain and compile
the task program.

## Deploy locally

After cloning the repository, run these commands from its root.

Create the local configuration:

```bash
cp .env.example .env
```

Replace `POSTGRES_PASSWORD` in `.env` with a URL-safe local password, then
check the tools and configuration:

```bash
cargo xtask stack doctor
```

Build and start the containers:

```bash
cargo xtask stack start
```

Confirm that the API is running:

```bash
curl -i http://localhost:8000/health_check
```

A healthy API returns `200 OK`. If you changed `APP_PORT`, use that port in the
health-check URL. To add repeatable sample players and device
logs:

```bash
scripts/seed_db.sh
```

The seed creates a development event and selects it only if no event is active.
It adds Hunter and Bounty teams with stable UUIDs; reseeding preserves their
names and existing player memberships.

Players are registered and listed within the active event. A fresh, unseeded
database has no active event: player registration returns `409 Conflict` and
player listing is empty. Create events with `POST /events` and select one with
`PUT /events/{id}/active`. New events have no teams.
List or create teams with `GET` or `POST /events/{event_id}/teams`; rename one
with `PATCH /events/{event_id}/teams/{id}`. Creation and renaming accept JSON
`{"name": "Runners"}`. After trimming, names must contain 1–256 Unicode scalar
values and no null characters. Names are case-sensitive unique within each
event (duplicates return 409).
Team routes use the explicit event, not the active selection. Renaming preserves
the team's UUID and memberships. Registration leaves `team_id` null, which is
also how unassigned players appear in listings.
These routes do not provide deletion or membership changes.
Registration also returns `409 Conflict` for a duplicate name within the active
event or exhausted PDN codes. Codes are allocated sequentially, skipping reserved
and already assigned values.

## API documentation

Open `/docs` on the running server to browse the API in ReDoc. The OpenAPI
specification is available at `/openapi.json` (the AI friendly format).

## Database documentation

The auto-generated [database schema reference](docs/database/README.md) is primarily
for AI consumption at the moment. It documents tables, relationships, and
constraints to help coding agents understand the current database structure.

## Routine commands

### Identify a deployment

`GET /version` returns the deployed commit SHA and its GitHub URL. Missing commit
metadata returns null for both fields; a missing repository URL returns null for
`commit_url` only.

### Manage the local stack

```bash
# Follow API logs.
cargo xtask stack logs

# Stop the containers while preserving the database.
docker compose stop

# Build and start (checks whether a database reset is required).
cargo xtask stack start
```

`stack logs` streams raw app logs without Compose colors or prefixes. You can
pipe them to a tool of your choice. For readable formatting with Bunyan:

```bash
cargo install bunyan # Optional, one-time installation.
cargo xtask stack logs | bunyan
```

Press Ctrl+C to stop following logs; the app keeps running.

`stack start` checks prerequisites, builds the images, and starts PostgreSQL to
check migration status before starting the rest of the stack. A fresh database
is migrated automatically. Startup uses SQLx statuses: only pending migrations
are treated as fresh; a mix of installed and pending migrations blocks startup
and asks you to run `cargo xtask stack db reset`.
If the database is up to date, startup preserves its data.
After pulling or changing application code, run:

```bash
cargo xtask stack start
```

PostgreSQL data remains in a Docker volume when the containers stop.
To delete all local database data, rebuild the stack, apply all migrations, and
load sample data, run:

```bash
cargo xtask stack db reset
```

This requires typing `reset` to confirm and does not make a backup. Use it only
for disposable local data. Updates with new migrations require this reset;
updates without pending migrations only need `cargo xtask stack start`.

`scripts/delete_local_instance.sh` asks for confirmation, backs up logs and the
database, and then deletes the local containers and database volume.

## Remote deployment and database

Set `REMOTE_DB_URL` (the DigitalOcean PostgreSQL connection URL, including its
SSL options) and `REMOTE_DEPLOY_URL` (the deployed API base URL) in `.env`.
Shell variables override `.env`; cloud commands never use the local
`DATABASE_URL` or `POSTGRES_*` settings.

```bash
cargo xtask cloud status      # Check the deployed API's health (requires curl).
cargo xtask cloud db info     # Show installed and pending migrations.
cargo xtask cloud db migrate  # Apply pending migrations without resetting data.
```

Remote database commands require Docker. They build `Dockerfile.migrations`
using cached layers, then run its tools against `REMOTE_DB_URL`. The same
Dockerfile serves local Compose migrations; the application Dockerfile no
longer includes the migration tools. Connection credentials are passed at
runtime, not baked into the image.

For disposable remote databases only:

```bash
cargo xtask cloud db seed   # Insert or update sample data.
cargo xtask cloud db reset  # Delete application data and reapply migrations.
```

Both commands require typing the database name to confirm. For automation,
set `CONFIRM_REMOTE_DB_SEED` or `CONFIRM_REMOTE_DB_RESET` in the shell to the
exact database name. Reset does not back up or seed data. Never use reset
for routine deployment or verification.

## Native development

API database connections limit each lock wait to 3 seconds and each SQL statement
to 10 seconds. PostgreSQL terminates connections left idle in an open transaction
for 30 seconds. These defaults are set in `src/startup.rs` for the API pool;
migration and maintenance connections do not inherit them from the application.

For API development outside Compose, initialize the standalone development
database and run the server:

```bash
scripts/init_db.sh
cargo run
```

Run the test suite with:

```bash
cargo test
```
