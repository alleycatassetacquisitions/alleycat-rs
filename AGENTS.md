# Repository guidance

## Old codebase

- For this project, "the old codebase" refers to the Hono/Bun server in
  `../alleycat-server-bun` (`/Users/danshari/projects/alleycat-server-bun`).
- Its repository is https://github.com/alleycatassetacquisitions/alleycat-server-bun.
- When working on this project, keep a local checkout of that repository alongside
  this one at `../alleycat-server-bun`; clone it there if it is missing.
- Use it as the reference implementation when investigating existing behavior
  or porting features to this Rust server.

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
