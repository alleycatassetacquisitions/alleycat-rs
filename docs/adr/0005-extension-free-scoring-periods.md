# 0005: Extension-free scoring-period constraints

- Status: Accepted
- Date: 2026-10-16

## Context

The scoring-period migration uses `btree_gist` to compare event UUIDs in an
exclusion constraint. The App Platform dev database has the extension available,
but its application role cannot install it: the role can create objects in
`public`, but lacks database-level `CREATE` privileges.

We need to keep the no-overlap rule from [0001](0001-quickdraw-results-and-scoring-periods.md)
without requiring a hosting upgrade or implementing concurrency control ourselves.

## Decision

Keep the PostgreSQL exclusion constraint, using a custom UUID range type and
the built-in GiST range operators. Represent an event as an inclusive range whose
two endpoints are that event's UUID. These singleton ranges overlap exactly when
the UUIDs match. Combined with the existing time-range comparison, this retains
the same per-event overlap rule, including for concurrent writes and updates.

Keep a separate B-tree index on the event UUID for application queries, which
filter on the UUID rather than the range expression. No API or scoring behavior
changes.

Generate database documentation with migrations executed by a role that has only
schema-level creation privileges. This verifies fresh installation without
depending on extension-installation privileges.

## Alternatives considered

- Install `btree_gist` through a database administrator or upgrade the database:
  the configured dev database does not provide the required privileges, and
  upgrading would introduce an additional recurring cost.
- Replace the exclusion constraint with triggers and explicit locking: this adds
  concurrency logic and more isolation-level concerns than retaining PostgreSQL's
  native enforcement.
- Enforce the rule only in the API: direct database writes could bypass it.

## Consequences

Fresh installations no longer need `btree_gist`. The UUID range representation
is less conventional than UUID equality and needs an explanatory SQL comment.
The custom range has no subtype-distance function; PostgreSQL still enforces
the constraint correctly, but GiST index efficiency should be revisited if
scoring-history volume makes writes expensive.

This change is an explicitly approved, pre-release exception to published
migration immutability: it replaces the body of migration `20261011120000`.
A later migration alone cannot fix a fresh installation that fails earlier.
The remote database had not applied this migration, so it can apply the revised
version without a reset. Local disposable databases that applied the original
version must be rebuilt before running migrations again; their SQLx checksum
will differ. Do not bypass checksum validation or reset a database with data
that needs preserving. Such a database requires a separately planned transition.

The reset inventory removes the application's range type and its dependent
multirange type. An existing `btree_gist` extension remains untouched because
unrelated objects may depend on it.

## References

- [Scoring-period migration](../../migrations/20261011120000_create_event_scoring_periods.sql)
- [Generated schema](../database/schema.sql)
- [Scoring-period reference](../database/schema/public.event_scoring_periods.md)
- [Database-boundary tests](../../tests/api/scoring.rs)
- [Documentation and reset checks](../../xtask/src/docs/README.md)
- [PostgreSQL range types](https://www.postgresql.org/docs/18/rangetypes.html)
