# Documentation maintenance

Database documentation is generated from migrations using
[tbls](https://github.com/k1LoW/tbls) for Markdown and Mermaid diagrams, and
`pg_dump` for the complete PostgreSQL schema. Migrations remain the source of truth.

## Generate and check

From the repository root, with Docker running:

```bash
cargo xtask docs db
```

Run the same consistency checks as CI without rewriting generated files:

```bash
cargo xtask docs db --check --check-reset
SQLX_OFFLINE=true cargo test --test documentation
```

The command starts a disposable PostgreSQL 18 container with no external network,
published ports, or persistent volumes. A version-pinned tbls container joins only
that database's network namespace. Docker downloads its image on first use; no
local tbls installation is needed. [`.tbls.yml`](../../../.tbls.yml) controls
presentation using upstream templates. The runner supplies the disposable database
DSN explicitly and never uses application database settings or `REMOTE_DB_URL`.

## Generated output

Commit regenerated changes under `docs/database/` alongside migration changes:

- `schema/` contains tbls pages and is entirely replaced on regeneration, including
  removal of obsolete table pages. Keep handwritten documentation outside it.
- `schema.sql` includes the full schema, including enum values and routines.

`--output PATH` places `schema/` and `schema.sql` under another directory.
`--check` compares the SQL dump and runs `tbls diff` with the documentation mounted
read-only.

## Reset verification

`--check-reset` creates another fresh database in the disposable container,
applies migrations, exercises `xtask/reset_remote_db.sql`, and checks that only
application objects and a SQLx ledger sentinel disappear. Unrelated objects,
data, and grants must survive, and migrations must apply again to produce the
same database. Migration SQL is applied directly; the ledger sentinel tests
table removal, not SQLx migration bookkeeping.

## API documentation tests

The documentation tests use the production router without a database. They check
OpenAPI references, operation IDs, path parameters, selected wire contracts, and
self-contained ReDoc assets. They run as part of `cargo test` in CI.
