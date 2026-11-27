# Database migrations

## Create a migration

Start Docker and make sure the Reylo 0.9.0 CLI is on your `PATH`.

1. Edit the [root schema.sql](../schema.sql) to describe the change.
2. From the repository root, run `cargo xtask db new <name>`. For example, after
   adding a `venue_notes` column to `events`, run
   `cargo xtask db new add_event_venue_notes`.
3. Review the generated SQL in `migrations/` and commit it with `schema.sql` and
   `reylo.lock`. Migrations already on `main` are immutable.

The command creates a disposable PostgreSQL 18 database, sets the connection
for Reylo, and generates the migration from your schema edit. Reylo replays
the full history, including the new migration, updates the lock, and checks the
result against your schema. The command removes the temporary database when it finishes. Application
deployments still use SQLx.

If the change adds, removes, or renames a table, view, sequence, routine, or
custom type, update [the reset inventory](../xtask/reset_remote_db.sql) and run
[the reset check](../xtask/src/db/README.md#check-the-database-reset-script).

## Correct a column name

If you just generated the latest migration on your branch and want to change
`venue_notes` to `venue_docs` before merging:

1. Change the column name in `schema.sql`.
2. Remove the newly generated `*_add_event_venue_notes.sql` file from
   `migrations/`. Restore `reylo.lock` to its state immediately before that
   migration. If the previous lock was committed and the new migration is
   uncommitted, use `git restore --source=HEAD -- reylo.lock`.
3. Run `cargo xtask db new add_event_venue_docs`. Review the SQL and updated lock.
   If you applied the old version to a separate disposable app database, rebuild
   that database before using the revised migration.

If the original migration has reached `main`, keep it. Change the column name
in `schema.sql` and generate a follow-up migration:

```bash
cargo xtask db new rename_event_venue_notes --rename events.venue_notes=events.venue_docs
```

The rename hint tells Reylo to generate a column rename that preserves existing values.

## Verify the committed migrations

To run the CI schema checks locally, set `DATABASE_URL` to a fresh disposable
PostgreSQL 18 database. Use a direct connection with permission to create shadow
databases. From the repository root, run Reylo 0.9.0:

```bash
reylo migrate deploy --dry-run
reylo migrate deploy
reylo migrate status
```

The dry run replays the migrations on a shadow database and compares the result
with the root schema. Deployment applies the locked migrations to the disposable
database. The status check verifies the database, migration files, and lock agree.
