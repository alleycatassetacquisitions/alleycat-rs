# 0002: Quickdraw device ingestion

- Status: Accepted
- Date: 2026-10-11

## Context

[ADR 0001](0001-quickdraw-results-and-scoring-periods.md) requires match-time
team snapshots and first-upload-wins results. Devices can upload after players
change teams or after an operator closes scoring. Current player membership
cannot reconstruct the original teams. The device-log API already uses protobuf
over HTTP, while the legacy match API used JSON.

## Decision

Devices upload protobuf batches with both players' team IDs captured at match
time. An absent team means unassigned. The server validates event ownership but
trusts the supplied snapshots, timestamps, measurements, and explicit winner.
It does not infer snapshots from current membership or infer winners from times.

Each batch targets an explicit event, so active-event changes cannot redirect
delayed uploads. Batches are atomic and bounded. Devices can retry an entire
batch after an uncertain response; accepted IDs preserve every original field.
An ID already owned by another event is a conflict, not a successful retry.

Ingestion and scoring controls serialize on the event row, then read period
history in a subsequent statement. This prevents reading stale period history
after waiting for an operator's scoring change. Inserts are ordered by match ID
to avoid conflicting batches acquiring those IDs in opposite orders. Stable
ordering retains the first report of a repeated ID within one batch.

## Alternatives considered

- Server-side membership history would permit reconstructing teams, but adds
  temporal membership data and update rules that are unnecessary when the
  devices already capture the teams at match time.
- JSON or dual-format ingestion would add a second device wire contract.
  Protobuf was selected to align with the existing device API.
- Partial batch success would require per-item acknowledgements and retry
  bookkeeping. Atomic batches keep delivery simple.

## Consequences

Firmware must retain event IDs, shared match IDs, and team snapshots alongside
results. The server cannot independently prove historical membership from these
snapshots. A failed batch must be corrected or split before retrying.

Uploads for one event serialize; independent events can proceed concurrently.
Revisit this if measured upload throughput requires finer-grained locking.
The trusted device-clock assumption and first-upload boundary tradeoff from
ADR 0001 still apply. Leaderboard formulas remain deferred.

## References

- [Device protobuf contract](../../proto/device_api.proto)
- [Quickdraw routes and OpenAPI annotations](../../src/routes/quickdraw.rs)
- [Scoring controls](../../src/routes/scoring.rs)
- [API documentation](../../readme.md#api-documentation)
- [Ingestion tests](../../tests/api/quickdraw.rs)
