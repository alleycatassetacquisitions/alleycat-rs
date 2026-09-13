# Script-to-xtask migration tracker

## Menu and submenu completion

Checked commands are implemented; unchecked commands still need work. Partial
implementations are labeled explicitly. Click a menu to open its dispatcher or a
command to open the file to implement. All menus are wired into the CLI; unfinished
commands return a clear error without taking action. Each command also links its
source script, or identifies the closest reference when no direct equivalent exists.
The deleted startup script links to its last version before removal.

Root dispatcher: [cargo xtask](xtask/src/main.rs).

- [native](xtask/src/native/mod.rs)
  - [ ] [doctor](xtask/src/native/doctor.rs) — check native prerequisites.
    No direct script equivalent.
  - [db](xtask/src/native/db/mod.rs)
    - [ ] [init](xtask/src/native/db/init.rs) — initialize standalone Postgres.
      Port [init_db.sh](scripts/init_db.sh).
    - [ ] [migrate](xtask/src/native/db/migrate.rs) — apply pending migrations.
      Extract the migration step from [init_db.sh](scripts/init_db.sh).
    - [ ] [seed](xtask/src/native/db/seed.rs) — seed the native database.
      Port the URL-based branch of [seed_db.sh](scripts/seed_db.sh).
    - [ ] [reset](xtask/src/native/db/reset.rs) — recreate the native database/container.
      Port [reset_db.sh](scripts/reset_db.sh).
    - [ ] [delete](xtask/src/native/db/delete.rs) — remove the container and volumes.
      Port [kill_db.sh](scripts/kill_db.sh).
    - [ ] [clean-tests](xtask/src/native/db/clean_tests.rs) — remove leftover test databases.
      Port [clean_test_dbs.sh](scripts/clean_test_dbs.sh).
- [stack](xtask/src/stack/mod.rs)
  - [x] [doctor](xtask/src/stack/doctor.rs)
    No direct script equivalent.
  - [x] [start](xtask/src/stack/start.rs)
    Ported from [start_local_instance.sh (before deletion)](https://github.com/alleycatassetacquisitions/alleycat-rs/blob/e4cea9d54dc8a43dfb794431884472c58fbe6368/scripts/start_local_instance.sh).
  - [ ] [delete](xtask/src/stack/delete.rs) — confirm, back up, and delete the stack/volume.
    Port [delete_local_instance.sh](scripts/delete_local_instance.sh).
  - [ ] [logs](xtask/src/stack/logs/follow.rs) — follow logs by default.
    Port [view_logs.sh](scripts/view_logs.sh).
    - [Log dispatcher](xtask/src/stack/logs/mod.rs)
    - [ ] [backup](xtask/src/stack/logs/backup.rs) — export app logs.
      Port [backup_logs.sh](scripts/backup_logs.sh).
  - [db](xtask/src/stack/db/mod.rs)
    - [ ] [migrate](xtask/src/stack/db/migrate.rs)
      Extract migration orchestration from [start_local_instance.sh (before deletion)](https://github.com/alleycatassetacquisitions/alleycat-rs/blob/e4cea9d54dc8a43dfb794431884472c58fbe6368/scripts/start_local_instance.sh) and [compose.yaml](compose.yaml).
    - [ ] [seed](xtask/src/stack/db/seed.rs) — **partial:** Compose works; script preflight parity remains.
      Port the Compose branch of [seed_db.sh](scripts/seed_db.sh).
    - [ ] [backup](xtask/src/stack/db/backup.rs)
      Port [backup_db.sh](scripts/backup_db.sh).
    - [ ] [reset](xtask/src/stack/db/reset.rs) — scope needs defining.
      No direct script equivalent; [delete_local_instance.sh](scripts/delete_local_instance.sh) deletes the entire stack/volume.
- [cloud](xtask/src/cloud/mod.rs)
  - [ ] [doctor](xtask/src/cloud/doctor.rs) — check the explicit cloud target.
    No direct script equivalent.
  - [db](xtask/src/cloud/db/mod.rs)
    - [ ] [migrate](xtask/src/cloud/db/migrate.rs)
      Extract only the migration/status steps from [reset_remote_db.sh](scripts/reset_remote_db.sh), without the reset.
    - [ ] [seed](xtask/src/cloud/db/seed.rs)
      Port the URL-based branch of [seed_db.sh](scripts/seed_db.sh).
    - [ ] [backup](xtask/src/cloud/db/backup.rs)
      No direct cloud script; adapt the dump/file handling in [backup_db.sh](scripts/backup_db.sh).
    - [ ] [reset](xtask/src/cloud/db/reset.rs)
      Port [reset_remote_db.sh](scripts/reset_remote_db.sh).
- [sqlx](xtask/src/sqlx/mod.rs)
  - [ ] [prepare](xtask/src/sqlx/prepare.rs) — refresh checked query metadata.
    Port the metadata step in [prepare_sqlx.sh](scripts/prepare_sqlx.sh), without its automatic reset.
  - [ ] [check](xtask/src/sqlx/check.rs) — check metadata without rewriting it.
    No script; reuse the SQLx freshness command in [general.yml](.github/workflows/general.yml).

Shared implementation files: [process helpers](xtask/src/command.rs),
[environment reader](xtask/src/env.rs), [repository paths](xtask/src/project.rs),
[seed runner](xtask/src/seed.rs), and [seed SQL](xtask/seed_db.sql).

Build/stop remain direct Compose commands; restore is a deferred procedure, not
a scaffolded command. Implemented commands still have follow-up improvements below.
URL-based seeding remains to be restored under native/cloud commands. Built-in `help` and
`--help` are supplied by Clap and omitted from this work checklist.

Last reviewed: 2026-09-13, branch `feature/tasks`, HEAD `0c74fcd`, including
the working-tree scaffolding added after the initial review.

This file tracks migration progress, remaining script dependencies, and follow-up
work. Run commands from the repository root. An implemented command means its
behavior exists in source; runtime verification is recorded separately below.
Backlog checkboxes mean the described work is complete, not merely that a command
name or module exists.

## Current progress

- The workspace, Cargo alias, and `native` / `stack` / `cloud` / `sqlx` menus exist.
- `stack doctor`, `stack start`, and Compose-only `stack db seed` retain their
  implementations. The standalone startup script has been removed.
- Native DB, cloud DB, stack DB, logs, stack deletion, and SQLx metadata commands
  are wired into the CLI. Each unfinished operation has its own file describing
  its responsibilities, followed by a minimal `run()` that returns an error.
- Native/cloud doctors and the existing stack DB placeholders now fail clearly
  instead of reporting success without performing the operation.
- The existing cloud DB dispatcher has been preserved and connected; its four
  missing implementation files now exist as stubs.
- Eleven shell scripts remain. The seed script still references its moved SQL
  file; the other scripts continue providing the legacy workflows.

The last substantive commits were `e4cea9d` (stack doctor), `c20d5fb` (stack
startup), and `0c74fcd` (DB command scaffolding and Compose seed implementation).
The working-tree scaffold adds structure only. Finish one Compose workflow at a
time, verify its behavior, then retire its script and update callers.

## Current xtask interface

Prefix each command below with `cargo xtask`. Inspect available commands with
`cargo xtask --help` and subcommand `--help`.

| Command | Status | Current behavior / limit |
| --- | --- | --- |
| `stack doctor` | Implemented | Checks Rust, Docker CLI, Compose, daemon, `.env`, and nonempty `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB`. Does not validate the rendered Compose configuration, database, or API health. |
| `stack start` | Implemented | Runs doctor, attempts to remove the old migration container, then runs `docker compose up -d`. Does not explicitly rebuild existing images or wait for API health. |
| `stack db seed` | Implemented, partial script parity | Streams [xtask/seed_db.sql](xtask/seed_db.sql) into Compose Postgres. Requires a running, migrated database. Does not support `DATABASE_URL` or retain the script's friendly preflight checks. |
| `stack db migrate` | Stubbed | Returns a not-implemented error. Startup still invokes the Compose migration service. |
| `stack db backup` | Stubbed | Returns a not-implemented error. Use the legacy backup script. |
| `stack db reset` | Stubbed | Returns a not-implemented error. Reset semantics still need defining. |
| `native doctor` | Stubbed | Returns a not-implemented error; no checks run. |
| `cloud doctor` | Stubbed | Returns a not-implemented error; no checks run. |
| `stack delete`, `stack logs`, `stack logs backup` | Stubbed | Wired to dedicated files; return not-implemented errors. |
| `native db init/migrate/seed/reset/delete/clean-tests` | Stubbed | Each operation has its own file; returns a not-implemented error. |
| `cloud db migrate/seed/backup/reset` | Stubbed | Each operation has its own file; returns a not-implemented error. |
| `sqlx prepare/check` | Stubbed | Separate metadata refresh and freshness-check files; return not-implemented errors. |

Slash-separated names above denote separate subcommands, not literal CLI syntax.
The startup log hint now resolves to a stub; following logs still requires the script.

## Script migration inventory

Replacement command names are scaffolded, but unfinished tasks do not yet replace
the scripts. Keep scripts available until replacement behavior and callers are verified.

| Legacy script / workflow | Current replacement or fallback | Remaining migration work |
| --- | --- | --- |
| `scripts/start_local_instance.sh` (deleted) | `cargo xtask stack start` | Update stale references in [readme.md](readme.md) and the seed script's error message. |
| [scripts/seed_db.sh](scripts/seed_db.sh) | `cargo xtask stack db seed` for Compose only | Script still reads missing `scripts/seed_db.sql`. Repair its path or provide a compatibility wrapper; restore URL-based seeding under an explicit environment command. |
| [scripts/backup_db.sh](scripts/backup_db.sh) | Keep using the script | Implement `stack db backup`; retain timestamped SQL in `backups/db/` and restrictive file permissions. |
| [scripts/backup_logs.sh](scripts/backup_logs.sh) | Keep using the script | Add log backup, for example `stack logs backup`; retain timestamped JSONL in `backups/logs/`. |
| [scripts/view_logs.sh](scripts/view_logs.sh) | Keep using the script | Add `stack logs` with Bunyan when available and raw-log fallback. |
| [scripts/delete_local_instance.sh](scripts/delete_local_instance.sh) | Keep using the script | Add a destructive stack-delete task. Preserve the actual order: confirmation, log backup, DB backup, then Compose teardown with volumes. Abort if either backup fails. |
| [scripts/init_db.sh](scripts/init_db.sh) | Keep using the script | Add native DB initialization: standalone Postgres, app role with `CREATEDB`, database creation, migrations. Preserve `SKIP_DOCKER=true` for CI and externally managed Postgres. |
| [scripts/reset_db.sh](scripts/reset_db.sh) | Keep using the script | Add native reset. Current behavior destroys and recreates the standalone container, not just the application schema. |
| [scripts/kill_db.sh](scripts/kill_db.sh) | Keep using the script | Add explicit native container deletion; current behavior force-removes container `postgres` and its anonymous volumes. |
| [scripts/clean_test_dbs.sh](scripts/clean_test_dbs.sh) | Keep using the script with care | Add native test-DB cleanup with an exact prefix match and target preview; see review findings below. |
| [scripts/prepare_sqlx.sh](scripts/prepare_sqlx.sh) | Keep using the script with care | Add SQLx metadata preparation. Current script destroys/recreates the native DB before `cargo sqlx prepare --workspace -- --all-targets`. |
| [scripts/reset_remote_db.sh](scripts/reset_remote_db.sh) | Keep using the script only for disposable remote DBs | Add cloud reset with explicit target, confirmation, narrowly scoped object inventory, migrations, and migration status. Never use it for routine verification. |

## Priority backlog and review findings

### 1. Repair the transition gaps

- [x] Make unimplemented commands fail with a clear message. Each stub now
  reports its full command name and exits unsuccessfully without taking action.
- [ ] Fix the seed script's moved SQL path and its deleted startup reference.
  Keep one fixture file shared by all seed entry points.
- [ ] Update [readme.md](readme.md) to use `cargo xtask stack start` and the working Compose
  seed task. Create `.env` before the first full doctor check; doctor currently
  requires it. Correct the documented deletion order to confirmation before
  backups.
- [ ] Implement `stack logs` or change the startup hint to the existing log
  script until log following is implemented.

### 2. Finish the Compose workflow

- [ ] Implement explicit migration execution with exit-status propagation.
  Rebuild the migration image after migration-file changes; migrations are
  copied into that image, not mounted from the checkout.
- [ ] Implement database and log backups, then the destructive delete workflow.
  Write backups to temporary files and publish the final name only after success,
  so a failed dump cannot look like a usable backup. Retain private permissions.
- [ ] Define `stack db reset`: application database/schema reset and deleting the
  entire Compose volume have different consequences. Document the chosen scope
  and target before implementing it.
- [ ] Decide whether `stack start` should wait for health and offer an explicit
  build option. Today it starts containers and prints a health-check command;
  successful exit does not establish API readiness.
- [ ] Add a restore procedure and verify it against a disposable database before
  relying on backups for destructive operations. Restore is new scope beyond
  script parity, but closes a real gap in the current lifecycle.

### 3. Port native and SQLx workflows

- [ ] Implement native doctor checks needed by each operation: tooling,
  container/port conflicts, database target, connectivity, and migration status.
  Add bounded readiness waits; `init_db.sh` currently polls health indefinitely.
- [ ] Port initialization, reset, deletion, test-DB cleanup, and URL-based seed.
  Preserve documented overrides and CI's `SKIP_DOCKER=true` behavior.
- [ ] Correct the cleanup predicate: SQL `LIKE 'alleycat_test_%'` treats both
  underscores as single-character wildcards. Match the literal `alleycat_test_`
  prefix used in [tests/api/helpers.rs](tests/api/helpers.rs), retain identifier quoting, and avoid
  selecting unrelated databases.
- [ ] Separate SQLx metadata refresh from destructive DB reset. Provide a
  read-only freshness check; make rebuilding a disposable DB an explicit action.
- [ ] Replace both `init_db.sh` calls in [.github/workflows/general.yml](.github/workflows/general.yml) before
  retiring that script. CI test and coverage jobs both depend on it.

### 4. Port cloud workflows deliberately

- [x] Connect the cloud dispatcher and add the four missing modules as compiling
  stubs. Their database behavior remains to be implemented.
- [ ] Implement cloud backup, migration, seed, and reset as needed. Cloud seed
  restores the legacy seed script's URL-based behavior.
- [ ] Implement operation-specific cloud doctor checks: explicit database/app
  target, required tools, authentication where needed, spec validation, and
  migration status without modifying shared state.
- [ ] Preserve the remote reset inventory rule in [AGENTS.md](AGENTS.md).
  The current script lists all four application tables, both custom types, and
  SQLx's ledger. Keep it synchronized with migrations while the script remains;
  update repository guidance when an xtask takes over. Never replace this with
  a blanket drop of provider-owned `public` or all objects owned by the user.
- [ ] Parse the remote URL and validate its hostname; the current script only
  checks whether the entire string contains `ondigitalocean.com`. Preserve
  database-name confirmation, avoid exposing credentials, and keep reset out of
  routine deployment and verification.
- [ ] Document the cloud deployment and post-deploy checks. [spec.yaml](spec.yaml) declares
  auto-deploy from `main`, but this checkout has no deployment xtask or explicit
  cloud migration job. Verify health and a database-backed endpoint after deploy.

### 5. Complete the migration consistently

- [ ] Resolve paths from the repository root throughout xtask. Most execution
  uses `project_root()`, but stack doctor reads `.env` from the working directory.
  Define environment-variable precedence consistently with Compose; the current
  `.env`-only validation and displayed port can disagree with shell overrides.
- [ ] Share process execution, target resolution, and error handling where useful;
  keep environment-specific orchestration explicit. Every failed subprocess
  should produce a failing task exit status.
- [ ] Cover xtask explicitly in CI. Plain `cargo test` and `cargo clippy` select
  only the root package in this workspace. Use `--workspace` or a dedicated
  `-p xtask` check; coverage and SQLx preparation already use `--workspace`.
- [ ] Add focused verification for target selection, subprocess failures,
  confirmation refusal, backup failure before deletion, and CI initialization.
  Use disposable local databases for lifecycle integration checks.
- [ ] Retire scripts only after replacement behavior, README references, CI
  callers, and script-to-script calls are migrated. A temporary wrapper is fine;
  duplicating implementations indefinitely is not necessary.

## Existing checks and optional future automation

These are useful repository workflows, but they do not all need xtask wrappers.
Single Cargo commands can remain direct commands. The previous checklist marked
several checks as uncovered even though GitHub Actions already runs them.

| Workflow | Current command / coverage |
| --- | --- |
| Setup | Install Git, Rustup, Docker/Compose; create `.env` from `.env.example`. Native DB work also requires SQLx CLI; URL-based seed/reset requires `psql`. |
| Build / health / stop | `docker compose build`; `curl -i http://localhost:8000/health_check` (use configured `APP_PORT`); `docker compose stop`. No dedicated xtasks. |
| Native API | `cargo run` after [scripts/init_db.sh](scripts/init_db.sh). |
| Tests | `cargo test`; CI test job exists. Add workspace/xtask coverage as described above. |
| Formatting | `cargo fmt --all` / `cargo fmt --all --check`; CI currently uses `cargo fmt --check`. |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings`; CI currently checks the root package with `SQLX_OFFLINE=true`. |
| SQLx freshness | `cargo sqlx prepare --workspace --check -- --all-targets`; already in CI, requires a migrated database. |
| Coverage | `cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info`, then `cargo llvm-cov report --html --output-dir coverage`; already in CI. |
| Dependency advisories | `cargo deny check advisories`; automated on a schedule and manifest/lockfile pushes in [.github/workflows/audit.yml](.github/workflows/audit.yml). |
| Protobuf lint | `buf lint`; no corresponding CI step or xtask. |

For schema and contract changes, continue reviewing migrations, testing a fresh
disposable database, planning data preservation where required, and keeping
[proto/device_api.proto](proto/device_api.proto), routes, tests, and [docs/api-contract.md](docs/api-contract.md) synchronized.
Before sharing changes, review the diff, run applicable checks, and confirm CI.

## Verification for this review

- Passed `SQLX_OFFLINE=true cargo build --locked --offline --workspace` after
  scaffolding; both the application and xtask compile.
- Passed `cargo clippy --locked --offline -p xtask --all-targets -- -D warnings`.
- Passed `cargo fmt --package xtask --check` and `git diff --check`.
- Checked all nine help menus and invoked all 20 unfinished commands with an
  empty `PATH`. Each returned exit code 1 with its expected not-implemented error.
- Inspected scripts, task implementations, Compose/Docker configuration, README,
  CI, migration object definitions, and branch history. Cargo metadata confirmed
  that the default workspace member is only the root application package.
- No database mutation, backup, reset, container startup, or deployment was run.
  Lifecycle behavior above is source-reviewed, not an end-to-end runtime claim.
