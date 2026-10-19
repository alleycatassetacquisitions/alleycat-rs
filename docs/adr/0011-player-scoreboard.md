# 0011: Player scoreboard

- Status: Accepted
- Date: 2026-10-18

## Context

We want to display player rankings and quickdraw statistics. Match records already
contain the winner, each player's draw time, and the accepted scoring decision.

## Decision

Calculate the scoreboard from saved matches, as planned in [ADR 0001](0001-quickdraw-results-and-scoring-periods.md).
Use only matches accepted as scoring for points, average draw time, and match
count. This keeps all three measures about the same games. Each win earns one
point; averages include the player's own measurements from wins and losses.

Expose ranked player results at `GET /score/players`. The `/score` namespace
groups score summaries, with `/players` identifying whose results are returned.

Rank individual players by score alone. Equal scores share a rank; average draw
time does not decide who wins a score tie. Include players without scoring
matches and inactive registrations so registration changes cannot hide earned
results.

Show each player's current team alongside their results. Changing or clearing
that membership changes the label, not the player's earned points. Historical
team snapshots and saved scoring decisions continue to determine which matches
count.

Use the active event's full match history. Query the records when requested;
do not add stored totals or a general statistics framework for future features.

## Alternatives considered

- Including practice and teammate matches in averages would show broader playing
  history, but would make the average describe a different set of matches from
  the score. That belongs in a future personal statistics feature.
- Using average draw time to break ties would add a second competitive rule.
  Shared ranks keep the scoreboard focused on wins.
- Storing running totals would complicate ingestion and require keeping derived
  values in sync. Revisit this only if measured query performance requires it.

## Consequences

Eligible late uploads appear on the next read and can change ranks and page
contents. A retry cannot add points twice because the scoreboard uses the single
accepted record per match. Current scoring settings never reinterpret saved
results.

The displayed team describes where the player belongs now; it does not imply
that all their points were earned on that team. Future team totals must use
match-time team snapshots.

## References

- [Scoreboard route and OpenAPI contract](../../src/routes/score.rs)
- [Scoreboard integration tests](../../tests/api/score.rs)
- [Active-event scope](0007-active-event-api-scope.md)
- [Delayed scoring](0003-reversible-delayed-scoring.md)
