# Developer guide

## Workflows

| Goal | Run code as | Database | Start here |
| --- | --- | --- | --- |
| Fast API/UI development and tests | Host Rust and Node processes | Standalone Docker container named `postgres` | [Native development](#native-development) |
| Exercise release images and service wiring | Docker Compose services | Compose-managed named volume | [Production-like local stack](operations.md#production-like-local-stack) |
| Share work over the internet | DigitalOcean App Platform | DigitalOcean managed PostgreSQL | [Shared DigitalOcean environment](operations.md#shared-digitalocean-environment) |

## Three databases
* Standalone `postgres` container for native development
* Compose-managed `alleycat-rs` volume for production-like local stack
* DigitalOcean managed PostgreSQL for shared dev environment

## Requirements

### All workflows

- Git
- Docker with the `docker compose` subcommand

### Native development

- Rustup. [`rust-toolchain.toml`](../rust-toolchain.toml) selects Rust `1.95.0`
  with Clippy and rustfmt. Native development, CI, and release images therefore
  use the same compiler version.
- `sqlx-cli` compatible with SQLx `0.8`:

  ```bash
  cargo install --version='~0.8' sqlx-cli \
    --no-default-features --features rustls,postgres
  ```

- Node.js `>=24.3.0` and npm for `web/`.
- A working platform C linker. `.cargo/.config.toml` contains optional
  LLVM/lld suggestions, but Cargo does not load that nonstandard filename.
- `psql` only when running `seed_db.sh` against an explicit `DATABASE_URL`,
  including the standalone native database or a remote database.

## Native development

Rust and Node run on the host; PostgreSQL runs in Docker.

### First setup

From the repository root:

```bash
scripts/init_db.sh
(cd web && npm ci)
```

`init_db.sh` starts and migrates a `postgres` container on `localhost:5432`
with these local-only accounts:

| Role | User | Password | Use |
| --- | --- | --- | --- |
| PostgreSQL superuser | `postgres` | `password` | Native API defaults and test database creation |
| Application user | `app` | `secret` | Owns the native development database |

The script traces commands; never pass it shared credentials.

The script expects that no container named `postgres` already exists. If the
container exists but is stopped, preserve its data and start it with:

```bash
docker start postgres
```

`docker start` does not apply migrations. After pulling new migrations, run
from the repository root:

```bash
DATABASE_URL=postgres://app:secret@localhost:5432/alleycat \
  sqlx migrate run
```

### Run the application

In one terminal:

```bash
cargo run
```

In another:

```bash
cd web
npm run dev
```

Native URLs are:

- UI: <http://localhost:44100>
- API: <http://localhost:8000>
- Health check: <http://localhost:8000/health_check>

The UI watches TypeScript files; restart `cargo run` after Rust changes. For
nondefault ports or API origins, see
[web runtime configuration](../web/README.md#runtime-configuration).

### Seed native development data

`seed_db.sh` targets Compose unless `DATABASE_URL` is set. To seed native
PostgreSQL, with `psql` installed:

```bash
DATABASE_URL=postgres://app:secret@localhost:5432/alleycat \
  scripts/seed_db.sh
```

The repeatable seed refreshes five players and 12 device reports.

### Pause or remove the native database

Preserve the database while stopping it:

```bash
docker stop postgres
```

`scripts/kill_db.sh` removes the container and all of its data. The native
container has no persistent named volume. `scripts/reset_db.sh` removes it and
creates a fresh migrated database.

## Tests and checks

With the standalone `postgres` container running, run all core CI checks:

```bash
cargo fmt --check
SQLX_OFFLINE=true cargo clippy -- -D warnings
cargo test
DATABASE_URL=postgres://app:secret@localhost:5432/alleycat \
  cargo sqlx prepare --workspace --check -- --all-targets
(cd web && npm test && npm run typecheck)
```

Each API test creates a fully migrated database named
`alleycat_test_<uuid>`. Test databases are not removed automatically. Before
using the cleanup helper, read its wildcard caveat in the
[Script reference](scripts.md), then run:

```bash
scripts/clean_test_dbs.sh
```

Set `TEST_LOG=1` to print structured API logs during a test run:

```bash
TEST_LOG=1 cargo test -- --nocapture
```

The SQLx check uses the migrated native database. GitHub Actions currently
fails to persist this URL between setup and the freshness check. CI also runs
`cargo llvm-cov`; run it locally for larger changes.

## Database and SQLx workflow

Migrations are ordered SQL files in `migrations/`. Treat an applied migration
as immutable; add a new migration for the next schema change.

Create and edit a migration:

```bash
sqlx migrate add <short_description>
```

Then regenerate checked query metadata and validate the project:

```bash
scripts/prepare_sqlx.sh
cargo test
git status --short
```

`prepare_sqlx.sh` destroys and recreates the native database, applies
migrations, and rewrites `.sqlx/`; it does not touch Compose data.

Run it after:

- adding or changing a migration;
- adding or changing a compile-time-checked `query!` or `query_as!` call; or
- upgrading SQLx in a way that changes its offline metadata.

When a migration changes a table, view, sequence, routine, or custom type,
update `reset_remote_db.sh`'s explicit inventory. Never broaden it to
DigitalOcean's `public` schema or use the remote reset for verification.

## Where to make a change

| Goal | Start | Keep in sync / verify |
| --- | --- | --- |
| Add or change an API route | `src/startup.rs`, then `src/routes/` | API tests and the [HTTP surface](architecture.md#http-surface) |
| Change player validation | `src/domain/` | Domain and API tests |
| Add a UI page | `web/app/routes.ts` and `web/app/actions/` | [Web page checklist](../web/README.md#add-a-page) |
| Change a device message | `proto/device_api.proto`, then its route | API tests, [Device API](device-api.md), and firmware rollout |
| Change the schema | A new file under `migrations/` | `prepare_sqlx.sh`, tests, and reset inventory |
| Change local topology | `compose.yaml` and Dockerfiles | Rebuild and run Compose |
| Change shared topology | `spec.yaml` | [Validate and apply the app spec](operations.md#app-spec-changes) |

## Troubleshooting

| Symptom | Likely cause | Check or fix |
| --- | --- | --- |
| `init_db.sh` says the name `postgres` is already in use | The native DB container already exists | Use `docker start postgres`, then apply pending migrations; or intentionally reset it |
| Port `5432` is occupied | Another host or Docker PostgreSQL is published there | Stop it, or pair the setup `POSTGRES_PORT` with `APP_DATABASE__PORT` for Rust |
| Native UI is not on port `3000` | `web/server.ts` defaults native runs to `44100` | Open `44100` or set `PORT=3000` |
| Native UI returns `502` | Rust API is down, unreachable, or returned the wrong JSON shape | Check `cargo run`, `API_ORIGIN`, and API logs |
| Compose changes do not appear | `start_local_instance.sh` starts existing images | Run `docker compose build` before starting |
| `cargo test` leaves many databases | Tests isolate themselves but do not tear databases down | Run `scripts/clean_test_dbs.sh` against the native container |
| Health check passes but pages fail | Health check does not query PostgreSQL | Inspect API logs and database health |
| SQLx reports stale or missing query data | Schema/query changed without refreshing `.sqlx/` | Run `scripts/prepare_sqlx.sh`, knowing it resets native data |
| API binds only to loopback | `APP_ENVIRONMENT` is unset or `local` | Containers set production mode; use an explicit override only when intended |

The [Script reference](scripts.md) explains each helper's effects and safety.
