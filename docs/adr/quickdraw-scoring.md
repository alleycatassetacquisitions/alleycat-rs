# Quickdraw scoring

- Status: Accepted

Determine scoring eligibility from device match time and scoring-period history.
A current on/off flag cannot classify an offline match. Trust synchronized device
clocks, accepting that small differences near a boundary can make eligibility
depend on which completed report is accepted first.

Let operators disable points from periods that have already closed without
permanently finalizing an event. Keep those matches without points rather than
reject them and leave devices retrying valid records. Period closure supplies a
cutoff without introducing a separate maximum upload age.

Preserve the scoring decision and match-time team credit once a result is accepted
so later settings and membership changes do not rewrite earned results. Turning
late scoring back on therefore affects only results not yet accepted; it cannot
restore points previously withheld. Corrections or voiding need their own agreed
rules. Keep completed non-scoring matches for history; ties contribute to no
scoreboard statistic.

Read game state from a single database snapshot so the active event, settings,
and server time describe the same instant.

## References

- [Scoring controls](../../src/routes/scoring.rs)
- [Game state contract](../../src/routes/game_state.rs)
- [Ingestion and scoring tests](../../tests/api/quickdraw.rs)
- [Scoring-period database constraints](scoring-period-constraints.md)
- [SQL schema](../database/schema.sql)
