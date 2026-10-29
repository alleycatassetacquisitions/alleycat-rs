# Database backups and recovery

## Back up

With Docker running, use either command:

```bash
cargo xtask cloud db backup  # Uses REMOTE_DB_URL from the shell or .env.
cargo xtask stack db backup # Uses the running Compose database.
```

Backups are timestamped plain SQL files in `backups/db/`. Use `--output /path`
to choose another directory; relative paths resolve from the repository root.
They include the whole database: schema, data, sequence values and SQLx migration
history, including every device inspection. The backup role needs read access
to all tables. Only successful dumps receive a `.sql` filename; ignore `.partial`
files. Keep protected copies outside the database host.

## Option 1: Read directly

If the server goes down, open the `.sql` file in a text editor to read or search
the saved schema and data. No running server or database is needed. To query
the data or browse it as tables in pgAdmin, restore it into a separate database.

## Option 2: Restore

Create an empty PostgreSQL 18 database with the source's encoding and locale.
Keep the application and migration jobs disconnected; do not migrate or seed it.
Set `REMOTE_DB_URL` to this new database, then run:

```bash
cargo xtask cloud db restore /path/to/backup.sql
cargo xtask cloud db info
```

You can also load the SQL file in [pgAdmin](https://www.pgadmin.org/docs/pgadmin4/latest/restore_dialog.html)
with **Restore → Format: Plain**. Choose a fresh database; pgAdmin bypasses
xtask's empty-target check.

Use the matching application checkout to check migration status and recovered
records before reconnecting the application. The backup records PostgreSQL
versions, the operator's checkout revision and applied migration history.

For local recovery, start only PostgreSQL on a fresh Compose volume, wait for it
to be healthy, then restore:

```bash
docker compose up -d postgres
cargo xtask stack db restore /path/to/backup.sql
```

Restore refuses existing user objects, including empty tables, and rolls back
on SQL errors. Use trusted, unmodified backups. The restoring role needs permission
to create objects and becomes their owner. Roles, grants, tablespaces and
database-level settings are not restored; configure application access separately.
Cloud commands use PostgreSQL 18 clients; newer source servers and older restore
targets are unsupported.
