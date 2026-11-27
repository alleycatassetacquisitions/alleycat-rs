# Database migrations

Use Reylo 0.9.0 from the repository root to author migrations. Application
deployments still run the committed SQL files with SQLx.

## Set up a local database

Point `DATABASE_URL` at a disposable local PostgreSQL 18 database using a direct
connection with permission to create shadow databases. When you first create
that database, run `reylo migrate deploy` to apply the committed migrations.
You can reuse this database for later changes.

## Create a migration

With `DATABASE_URL` pointing at your local migration database:

1. Edit the [root schema.sql](../schema.sql) to describe the change. For
   example, after adding a `venue_notes` column to `events`, run
   `reylo migrate new add_event_venue_notes`. Review the SQL file it writes in
   `migrations/`. Migrations already on `main` are immutable.
2. Run `reylo migrate dev` to apply the new migration and update `reylo.lock`.
   Run `reylo migrate status` to confirm the migrations, schema, and lock agree.
3. Commit `schema.sql`, the new migration, and `reylo.lock` together. If the change
   adds, removes, or renames a table, view, sequence, routine, or custom type,
   update [the reset inventory](../xtask/reset_remote_db.sql) and run
   [the reset check](../xtask/src/db/README.md).

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
