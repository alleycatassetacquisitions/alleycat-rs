# Database maintenance

## Create a migration

Use Reylo 0.9.0 from the repository root to author migrations. Application
deployments still run the committed SQL files with SQLx.

1. Set `DATABASE_URL` to a fresh, disposable PostgreSQL 18 database. Use a direct
   connection with permission to create shadow databases. Run `reylo migrate deploy`
   to apply the committed migrations to it.
2. Edit the [root schema.sql](../../../schema.sql) to describe the change. For
   example, after adding a `venue_notes` column to `events`, run
   `reylo migrate new add_event_venue_notes`. Review the SQL file it writes in
   `migrations/`. Migrations already on `main` are immutable.
3. Run `reylo migrate dev` to apply the new migration and update `reylo.lock`.
   Run `reylo migrate status` to confirm the migrations, schema, and lock agree.
4. Commit `schema.sql`, the new migration, and `reylo.lock` together. If the change
   adds, removes, or renames a table, view, sequence, routine, or custom type,
   update [the reset inventory](../../reset_remote_db.sql) and run
   `cargo xtask db check-reset`.

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

## Reset verification

With Docker running, `cargo xtask db check-reset` starts an isolated PostgreSQL 18 container,
applies migrations, exercises `xtask/reset_remote_db.sql`, and checks that only
application objects and a SQLx ledger sentinel disappear. Unrelated objects,
data, and grants must survive, and migrations must apply again to produce the
same database. Migration SQL is applied directly; the ledger sentinel tests
table removal, not SQLx migration bookkeeping.
