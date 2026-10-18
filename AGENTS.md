# Repository guidance

## Implementation choices

- Prefer the simplest solution that preserves correctness. Check built-in
  capabilities before adding dependencies, custom logic, or infrastructure costs.

## Testing shared abstractions

- Test shared behavior at the abstraction boundary rather than repeating the
  same assertions at every call site. Adding a new use of an abstraction should
  not require adding it to a central test inventory.
- Add call-site tests for distinct behavior or meaningful integration risks;
  use representative coverage when multiple callers follow the same path.

## API documentation

- Follow [ADR 0007](docs/adr/0007-active-event-api-scope.md) when choosing event
  scope for routes. Routine operations use the active event; older management
  routes with explicit event IDs are pending repair, not templates to copy.
- Update OpenAPI annotations alongside API changes.
- Keep HTTP endpoint contracts in OpenAPI annotations and device message contracts
  in `.proto` comments. The generated API docs are the reference for consumers.
- Keep the README focused on setup, operations, and links to the generated docs.
  Do not duplicate endpoint inventories, request/response fields, payload examples,
  status-code tables, or validation rules there. Operational commands such as a
  deployment health check are fine. When moving existing details out of the
  README, preserve any missing contract information in its authoritative source.
- Keep ReDoc self-contained: local assets and system fonts, no CDN dependencies.
- Keep device message fields and validation requirements in the `.proto` comments.
  After protobuf changes, run `cargo xtask docs proto` with Docker running and
  include the generated `static/device-apis/index.html`; do not edit it manually.

## Architecture decisions

- Record significant architecture decisions and their tradeoffs in `docs/adr/`.
  Follow its README and template; routine implementation changes do not need an ADR.
- Read relevant accepted ADRs before changing the behavior they describe.
- When a decision changes, add a superseding ADR and link both records rather
  than rewriting the original rationale. Keep the ADR index current.
- Keep API contracts in OpenAPI and schema details in generated database docs;
  link to them from ADRs instead of duplicating reference documentation.

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
  The complete migration set must still apply successfully to an empty database.
- Migrations already pushed to `main` are immutable: do not edit, rename, or
  delete them. Changes to that schema require new migrations.
- New migrations that have not reached `main` may be edited or consolidated
  within the current change. Check the current remote `main` before doing so.
  If an edited migration was already applied to a local development database,
  that database must be rebuilt before reapplying the revised migrations.
  Squashing published migration history remains a separate, explicitly requested
  maintenance task.
- Before release, revisit this policy. From release onward, schema changes must
  use forward migrations that preserve existing data unless explicitly agreed
  otherwise.
- Whenever a migration is added or changed, run `cargo xtask docs db` with
  Docker running and include any regenerated changes to
  `docs/database/schema/` and `docs/database/schema.sql` in the same change.
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
