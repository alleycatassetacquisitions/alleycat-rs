# 0003: Reversible delayed scoring

- Status: Accepted
- Date: 2026-10-15
- Supersedes: [0001](0001-quickdraw-results-and-scoring-periods.md) for delayed-match eligibility; its storage, timing, and historical-credit decisions remain in effect.

## Context

Operators need to stop awarding points for late uploads without permanently
finalizing an event. Previously, a match inside any historical scoring period
remained eligible regardless of when it arrived. Devices must still be able to
upload their records while delayed scoring is disabled.

## Decision

Each event has a reversible delayed-score switch, enabled by default. An upload
is delayed when its occurrence time belongs to a scoring period that is already
closed at ingestion. When the switch is off, such a new match is saved with the
distinct non-scoring status `delayed_scores_disabled`. Matches in the open period
remain eligible; matches outside all periods remain non-scoring.

The switch is read under the same event lock used for scoring-period changes
and ingestion. A subsequent statement observes the setting after acquiring the
lock. The first accepted report still fixes every stored field and the scoring
decision. Re-enabling affects only newly accepted matches, never existing records
or retries. Team snapshots and historical credit remain unchanged.

The game-status read uses one database statement to return a coherent snapshot
of the active event and its settings, with the database's UTC clock.

## Alternatives considered

- Permanent finalization would prevent operators from reversing the setting.
- Rejecting uploads would force devices to retain and retry otherwise valid data.
- Recomputing saved results when toggling would make scores change retroactively
  and break first-report-wins ingestion.
- An arbitrary age threshold would require a separate definition of lateness;
  closed scoring periods already establish the relevant boundary.

## Consequences

An operator can stop late points while continuing to collect match records.
Re-enabling does not restore points for matches accepted while the switch was
off. A future correction or adjudication workflow would be a separate decision.

## References

- [Game state routes and OpenAPI](../../src/routes/game_state.rs)
- [Ingestion](../../src/routes/quickdraw.rs)
- [Event schema](../database/schema/public.events.md)
- [Match schema](../database/schema/public.quickdraw_matches.md)
- [Ingestion consistency decision](0002-quickdraw-device-ingestion.md)
