# 0001: Quickdraw results and scoring periods

- Status: Accepted
- Date: 2026-10-11

## Context

Teams have replaced the legacy Hunter/Bounty distinction. Quickdraw is defined,
while other game modes are still being designed. Both devices independently
upload the same duel, sometimes late and with slightly different timestamps.
We trust their periodically synchronized UTC clocks.

## Decision

- **Use a dedicated quickdraw table.** Store both players, their teams at match
  time, draw times, and the winner. Design other modes when their rules are known.
- **Keep one result per shared match ID.** The first valid upload supplies the
  result and device timestamp. Later uploads neither replace it nor score again.
  Separate device reports are unnecessary for delivery redundancy.
- **Keep scoring-period history.** Turning scoring on starts a period; turning it
  off closes that period. Periods cannot overlap within an event. Opening counts;
  closing does not. Repeated on/off requests do nothing.
- **Use match time for eligibility.** Find the event's scoring period containing
  the saved device timestamp. A current on/off flag alone cannot classify delayed
  uploads. Keep server receipt time separately for troubleshooting.
- **Keep non-scoring matches.** Matches outside scoring periods, between teammates,
  or involving an unassigned player remain recorded but do not score.
- **Preserve historical credit.** Save team information from match time and the
  scoring decision, without retaining a scoring-period reference. Later membership
  changes or closing scoring do not change earlier results.
- **Calculate leaderboards from match records.** Those records let us verify and
  rebuild scores. Add saved totals later if performance requires them.

## Tradeoffs and deferred work

We accept that small timestamp differences near a scoring boundary can change
eligibility depending on which device uploads first. We do not retain both
reports for comparison; revisit this if reconciliation becomes necessary.

Routes are a separate change. They must select the correct scoring period and
obtain teams from match-time context; current membership cannot reconstruct a
team change before a delayed upload. The schema enforces event ownership, while
period lookup and duplicate handling belong to ingestion.

Leaderboard formulas still need to account for late arrivals, equal timestamps,
event-local days, and how non-scoring matches affect streaks. Posse support,
other modes, voiding, and award snapshots are deferred.

## References

- [Generated database schema](../database/schema.md)
- [Scoring period migration](../../migrations/20261011120000_create_event_scoring_periods.sql)
- [Quickdraw migration](../../migrations/20261011121000_create_quickdraw_matches.sql)
- [Database tests](../../tests/api/scoring.rs)
