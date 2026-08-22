# Repository workflow checklist

Use this file as both a working checklist and an automation backlog.

- `[x]` means repository automation already covers the workflow.
- `[ ]` means the workflow is still manual or its automation is incomplete.
- Run commands from the repository root unless an item says otherwise.

## Workflows covered by repository automation

### Environment and local lifecycle

- [x] Check the Rust, Docker, Docker Compose, Docker daemon, and `.env`
  requirements for the local Compose stack.
  - Run: `cargo xtask stack doctor`
- [x] Start or restart the local Compose stack and apply pending migrations.
  - Run: `scripts/start_local_instance.sh`
  - This starts existing images; build changed application code separately.
- [x] Initialize the standalone Postgres container for native development,
  create the application role and database, and apply migrations.
  - Run: `scripts/init_db.sh`
- [x] Recreate the standalone native-development database from scratch.
  - Run: `scripts/reset_db.sh`
- [x] Force-stop and delete the standalone native-development Postgres
  container and its anonymous volumes.
  - Run: `scripts/kill_db.sh`
- [x] Back up local data and logs, ask for explicit confirmation, and then
  delete the local Compose containers, network, and Postgres volume.
  - Run: `scripts/delete_local_instance.sh`

### Data, logs, and database maintenance

- [x] Back up the Compose database to a timestamped SQL file in `backups/db/`.
  - Run: `scripts/backup_db.sh`
- [x] Back up the Compose application logs to a timestamped JSONL file in
  `backups/logs/`.
  - Run: `scripts/backup_logs.sh`
- [x] Follow the Compose application logs, using Bunyan when it is available
  and falling back to raw logs when it is not.
  - Run: `scripts/view_logs.sh`
- [x] Seed repeatable sample players and device logs into the local Compose
  database or a database selected by `DATABASE_URL`.
  - Run: `scripts/seed_db.sh`
- [x] Terminate connections to and drop leftover `alleycat_test_*` databases.
  - Run: `scripts/clean_test_dbs.sh`
- [x] Rebuild the native-development database and refresh checked SQLx query
  metadata for the whole workspace.
  - Run: `scripts/prepare_sqlx.sh`
- [x] Destructively reset a disposable DigitalOcean database, reapply all
  migrations, and show migration status.
  - Run: `DATABASE_URL=... scripts/reset_remote_db.sh`
  - This permanently deletes application data and must not be used as routine
    verification.

## Workflows not covered by a script

### Initial setup and local operation

- [ ] Install the repository prerequisites: Git, Rustup, Docker with Compose,
  and SQLx CLI for native database work.
- [ ] Create `.env` from `.env.example` and replace the example Postgres
  password with a URL-safe local password.
  - Run: `cp .env.example .env`, then edit `.env`.
- [ ] Build or rebuild the Compose images before the first start and after
  application-code changes.
  - Run: `docker compose build`
- [ ] Verify that the local API is healthy after startup.
  - Run: `curl -i http://localhost:8000/health_check`
- [ ] Stop the Compose containers while preserving the database volume.
  - Run: `docker compose stop`
- [ ] Run the API natively after initializing the standalone database.
  - Run: `cargo run`
- [ ] Restore a database from a file created by `scripts/backup_db.sh`.
  - There is currently a backup workflow, but no restore script.

### Development checks

- [ ] Run the test suite.
  - Run: `cargo test`
- [ ] Format Rust code and verify formatting.
  - Run: `cargo fmt` and `cargo fmt --check`
- [ ] Run Clippy with warnings treated as errors.
  - Run: `cargo clippy -- -D warnings`
- [ ] Verify that checked SQLx query metadata is current without rewriting it.
  - Run: `cargo sqlx prepare --workspace --check -- --all-targets`
- [ ] Generate and inspect the code-coverage report.
  - Run: `cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info`
    and `cargo llvm-cov report --html --output-dir coverage`
- [ ] Check Rust dependencies for security advisories.
  - Run: `cargo deny check advisories`
- [ ] Lint the protobuf definitions.
  - Run: `buf lint`

### Database and contract changes

- [ ] Create and review a new migration when the database schema changes.
- [ ] When a migration adds, removes, or renames a table, view, sequence,
  routine, or custom type, update the explicit object inventory in
  `scripts/reset_remote_db.sh` in the same change.
- [ ] Exercise the migration against a disposable local database and confirm
  that a fresh database can be built from the complete migration history.
- [ ] Plan and test a non-destructive data migration when a schema change must
  preserve existing shared-environment data.
- [ ] Keep `proto/device_api.proto`, route behavior, tests, and the relevant API
  contract documentation synchronized when an endpoint changes.

### Native and cloud environment checks

- [ ] Check native-development tooling, repository configuration, port and
  container-name conflicts, database health, applied migrations, and cleanup
  targets before changing local state.
  - `cargo xtask native doctor` exists, but it does not implement these checks
    yet.
- [ ] Check cloud tooling and authentication, validate the DigitalOcean app
  spec and target, compare live and local configuration, inspect pending
  migrations, and confirm backup policy before changing shared state.
  - `cargo xtask cloud doctor` exists, but it does not implement these checks
    yet.
- [ ] Deploy through the configured DigitalOcean workflow and verify both the
  public health check and a database-backed endpoint afterward.
- [ ] Confirm that the unauthenticated API is only exposed in an acceptable
  environment; the current deployment configuration does not add
  authentication.

### Before sharing a change

- [ ] Review the working tree and diff for accidental files, secrets, or
  unrelated changes.
- [ ] Run the applicable test, formatting, lint, SQLx, protobuf, and security
  checks locally.
- [ ] Commit and push the change, open or update the pull request, and confirm
  that the GitHub Actions checks pass.
