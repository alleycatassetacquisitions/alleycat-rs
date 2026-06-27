# Repository guidance

## Database migrations

- `scripts/reset_remote_db.sh` resets the disposable DigitalOcean database by
  explicitly dropping every application schema object created by `migrations/`,
  plus SQLx's `_sqlx_migrations` ledger.
- Whenever a migration adds, removes, or renames a table, view, sequence,
  routine, or custom type, update the reset inventory in
  `scripts/reset_remote_db.sh` in the same change.
- Keep the reset explicit and narrowly scoped. Do not replace it with a generic
  drop of the provider-owned `public` schema or every object owned by the
  database user.
- Never run the remote reset script as part of routine verification; it
  permanently deletes all data in the target database.
