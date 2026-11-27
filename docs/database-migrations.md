# Database migrations

## Install the Reylo CLI

On macOS or Linux, install the version used by CI once on your machine:

```sh
curl -fsSL https://reylo.dev/install.sh | REYLO_VERSION=0.9.0 sh
export PATH="$HOME/.local/bin:$PATH"
reylo --version
```

Check that the output reports `0.9.0`. If your shell does not already include
`~/.local/bin` on its `PATH`, add the export line to your shell profile so
`cargo xtask` can find Reylo in future sessions. For Windows, see the
[Reylo CLI installation instructions](https://reylo.dev/cli).

## Create a migration

Start Docker before running the command.

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

## Correct a migration

First, fetch `origin/main` and check whether it contains the migration file.
Once a migration reaches `main`, its SQL and filename are permanent.

### Replace the latest migration on your branch

If the migration is the last one on your branch and has never reached `main`,
you can regenerate it, even if you already committed it locally.

1. Edit `schema.sql` to describe the intended result.
2. Remove the migration file you are replacing. Restore `reylo.lock` to the
   version that covers the migrations you kept. If the migration is uncommitted
   and the lock was clean before generation, use
   `git restore --source=HEAD -- reylo.lock`. For a committed migration, use
   the commit immediately before it was added as the restore source.
3. Run `cargo xtask db new <name>` with a name that describes the corrected
   change. Review the generated SQL, especially any bootstrap SQL or data
   updates you added by hand. Carry those forward if they are still needed.
4. Review and commit the replacement file, the removal of the old file,
   `schema.sql`, and `reylo.lock` together. If your disposable app database
   already ran the old migration, rebuild it before applying the replacement.

### Add a follow-up migration

If the migration is on `main`, keep it and [create a migration](#create-a-migration)
for the correction. A follow-up is also the simplest option when later migrations
on your branch depend on the original. Start from the current lock, edit
`schema.sql`, and run `cargo xtask db new <name>`.

For a table or column rename, pass `--rename OLD=NEW` so Reylo can preserve
existing values. For example, to rename a column:

```bash
cargo xtask db new rename_event_venue_notes --rename events.venue_notes=events.venue_docs
```

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
