# 0003: Reversible delayed scoring

- Status: Accepted
- Date: 2026-10-15
- Supersedes: [0001](0001-quickdraw-results-and-scoring-periods.md) only for deciding whether late uploads can score. Its decisions about storing matches, using match time, and preserving past results still apply.

## Context

Devices can upload a match after scoring has been turned off. Until now, that
match could still earn points as long as it happened while scoring was on.

We want operators to be able to turn those late points off and back on. They
don't need to permanently finalize an event, and we still want to keep the match
records even when they won't count toward the score.

## Decision

Each event gets an "accept delayed scores" switch, on by default. Operators can
change it independently of whether scoring is currently on.

For this switch, a match is late if it happened during a scoring period that
has ended by the time the server processes the upload. With the switch off, we
save the match as `delayed_scores_disabled` and award no points. Matches from the
currently open period can still score. Matches that happened while scoring was
off still don't count.

Once we've accepted a match, its details and scoring decision stay put. Turning
the switch back on only affects matches we haven't accepted yet. Uploading the
same match again won't change its points or which teams receive credit.

Uploads and changes to either scoring switch take the same lock on the event.
After getting that lock, an upload reads the settings in a separate database
statement. This matters when an upload has to wait for a setting change: it must
use the setting that was just saved, not the one from before it started waiting.

The game-state endpoint reads the active event, its settings, and the database's
UTC time in one query so the response describes a single snapshot.

## Alternatives considered

- Permanently finalizing an event would stop late points, but operators couldn't
  turn them back on.
- Rejecting late uploads would leave devices holding valid match records and
  retrying them. We'd rather save those records without awarding points.
- Recalculating past results whenever the switch changes would make existing
  scores change and undo our rule that the first accepted report determines the
  result.
- Using a time limit, such as a maximum upload age, would add another rule for
  operators and devices to understand. The end of a scoring period already gives
  us a clear cutoff.

## Consequences

Operators can stop late uploads from adding points without losing the records
of those matches.

The tradeoff is that turning the switch back on won't restore points for matches
accepted while it was off. If we later need a way to correct those results, we'll
design that separately rather than make this switch rewrite past scores.

## References

- [Game state routes and OpenAPI](../../src/routes/game_state.rs)
- [Ingestion](../../src/routes/quickdraw.rs)
- [Event schema](../database/schema/public.events.md)
- [Match schema](../database/schema/public.quickdraw_matches.md)
- [Ingestion consistency decision](0002-quickdraw-device-ingestion.md)
