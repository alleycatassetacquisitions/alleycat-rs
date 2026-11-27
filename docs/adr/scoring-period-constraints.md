# Scoring-period database constraints

- Status: Accepted

Enforce non-overlapping scoring periods with a PostgreSQL exclusion constraint
using singleton UUID ranges and built-in range operators. This preserves native
concurrency enforcement without requiring `btree_gist`, which the configured
App Platform development database's application role lacks permission to install.

Upgrading hosting would add recurring cost. Triggers and explicit locks would
introduce concurrency logic, while API-only checks would let direct database writes
bypass the rule. Verify fresh migrations with schema-level creation privileges
so documentation builds do not hide an extension-permission dependency.

The UUID range representation is less familiar than equality and needs a SQL
comment. Its lack of a subtype-distance function may affect index efficiency;
revisit that if scoring-history volume makes writes expensive.

## References

- [Quickdraw scoring](quickdraw-scoring.md)
- [Scoring-period migration](../../migrations/20261011120000_create_event_scoring_periods.sql)
- [SQL schema](../../schema.sql)
- [Database-boundary tests](../../tests/api/scoring.rs)
- [Schema verification](../database-migrations.md#verify-the-committed-migrations) and [reset checks](../../xtask/src/db/README.md)
