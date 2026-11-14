# Player scoreboard

- Status: Accepted

Calculate rankings from saved scoring matches so scores can be verified and
rebuilt without keeping running totals in sync. Add stored totals only if measured
query performance requires them.

Use the same eligible matches for points, match counts, and average draw times so
the measures describe the same games. Broader playing statistics would answer a
different question. Rank by wins alone and share ranks for equal scores; using
draw time to break ties would introduce another competitive rule.

Include inactive registrations so registration changes cannot hide earned results.
Show current team membership as a label; it does not determine past credit.
Any future team totals must use match-time team snapshots.

## References

- [Scoreboard route and OpenAPI contract](../../src/routes/score.rs)
- [Scoreboard integration tests](../../tests/api/score.rs)
- [Active-event scope](event-api-scope.md)
- [Delayed scoring](quickdraw-scoring.md)
