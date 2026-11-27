# Database tasks

Use `cargo xtask db new <name>` to [create a migration](../../../docs/database-migrations.md#create-a-migration)
from an edit to the root schema.

## Check the database reset script

Run this check after changing the [reset inventory](../../reset_remote_db.sql).
With Docker running, execute the command from the repository root:

```bash
cargo xtask db check-reset
```

### What it verifies

The command starts its own disposable PostgreSQL 18 container and checks that:

- The reset script removes the application objects created by the migrations.
- It also removes the SQLx migration history table. The check creates a stand-in
  for that table because it runs the migration SQL directly.
- Unrelated database objects, data, and grants survive the reset.
- Applying all migrations again recreates the same schema.
