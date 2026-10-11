# Repository guidance

## Testing shared abstractions

- Test shared behavior at the abstraction boundary rather than repeating the
  same assertions at every call site. Adding a new use of an abstraction should
  not require adding it to a central test inventory.
- Add call-site tests for distinct behavior or meaningful integration risks;
  use representative coverage when multiple callers follow the same path.

## API documentation

- Update OpenAPI annotations alongside API changes.
- Keep ReDoc self-contained: local assets and system fonts, no CDN dependencies.

## Old codebase

- For this project, "the old codebase" refers to the Hono/Bun server in
  `../alleycat-server-bun` (`/Users/danshari/projects/alleycat-server-bun`).
- Its repository is https://github.com/alleycatassetacquisitions/alleycat-server-bun.
- When working on this project, keep a local checkout of that repository alongside
  this one at `../alleycat-server-bun`; clone it there if it is missing.
- Use it as the reference implementation when investigating existing behavior
  or porting features to this Rust server.

## Database migrations

- Until the release version, treat migrations as applying to a fresh database.
  Schema changes do not need to preserve existing development data or include
  data backfills or data-preserving transition migrations.
  Always add new migrations for schema changes; do not modify existing migrations.
  This keeps unapplied-migration detection in the xtask stack startup workflow
  reliable. Squashing migrations is a separate, explicitly requested maintenance
  task, not part of routine schema changes.
  The complete migration set must still apply successfully to an empty database.
- Before release, revisit this policy. From release onward, schema changes must
  use forward migrations that preserve existing data unless explicitly agreed
  otherwise.
- Whenever a migration is added or changed, run `cargo xtask docs db` with
  Docker running and include any regenerated changes to
  `docs/database/README.md` and `docs/database/schema.sql` in the same change.
  Do not edit these generated files manually.
- `cargo xtask cloud db reset` resets the
  disposable DigitalOcean database using `xtask/reset_remote_db.sql`, by
  explicitly dropping every application schema object created by `migrations/`,
  plus SQLx's `_sqlx_migrations` ledger.
- Whenever a migration adds, removes, or renames a table, view, sequence,
  routine, or custom type, update the reset inventory in
  `xtask/reset_remote_db.sql` in the same change.
- Keep the reset explicit and narrowly scoped. Do not replace it with a generic
  drop of the provider-owned `public` schema or every object owned by the
  database user.
- Never run the remote reset command as part of routine verification; it
  permanently deletes all data in the target database.
