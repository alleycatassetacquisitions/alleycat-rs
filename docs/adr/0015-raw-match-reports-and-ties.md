# 0015: Retain raw match reports and completed ties

- Status: Accepted
- Date: 2026-11-13
- Supersedes: The decision not to retain per-device reports in [0001](0001-quickdraw-results-and-scoring-periods.md). Extends [0010](0010-partial-match-batch-acceptance.md) with raw-report retention; its completed-result atomicity remains in effect.

## Context

Retain raw reports to debug device, communication, and upload problems, and to
investigate the unlikely case of a bad actor submitting conflicting results.
Devices also need to report ties and failures that award no points.

## Decision

Keep every eligible submission, including duplicates and rejected entries,
alongside the deduplicated results. Both use the active event. The upload
contract defines which request-wide errors prevent raw storage.

Trust the reported outcome. Failures stay in raw history and leave the match ID
available for a later completed result. The first accepted completed result wins,
including ties. Ties have no winner and contribute to no scoreboard statistic.
Later disagreements leave saved results unchanged.

Store each known submitted field in a queryable column. Preserve original
identifier strings, absent versus empty values, and the full unsigned draw-time
range, including values rejected by completed-result validation. Keep parsed
match IDs and nonblank MAC strings alongside submitted values for grouping, plus the
accepted outcome and comparison key for disagreement reports.

Let PostgreSQL reject NUL characters in submitted strings, roll back the upload,
and return HTTP 400. Unknown protobuf fields are discarded.

Compare accepted outcomes, participant IDs, per-player draw times, and winners.
Normalize participant order. Ignore timestamps, teams, MACs, and failure text.
Count distinct nonblank MAC strings exactly as submitted across all submissions,
including rejected entries,
to identify matches with one reporter or no known reporter.

## Alternatives considered

Keeping only completed results loses debugging evidence. Comparing whole reports
flags expected differences. Inferring or reconciling outcomes would move game
rules into the server and could change saved scores. Binary payloads make SQL
analysis harder; storing both bytes and columns duplicates the data.

## Consequences

Storage grows with every upload; no automatic retention policy applies. A match
may appear orphaned until its second device reports, and optional MACs limit
reporter counts. Consumers must handle nullable winners. Firmware must mark
failures explicitly so they are not recorded as ties. Adding a protobuf field
requires updating the raw table, ingestion, and response mapping.

## References

- [Device contract](../../proto/device_api.proto)
- [Upload and match listing](../../src/routes/quickdraw.rs)
- [Raw report endpoints](../../src/routes/quickdraw_reports.rs)
- [Raw report schema](../database/schema/public.quickdraw_matches_raw.md)
- [Scoreboard decision](0011-player-scoreboard.md)
