# 0009: Accept match uploads only for the active event

- Status: Accepted
- Date: 2026-10-18
- Supersedes: The explicit-event upload behavior in [0002](0002-quickdraw-device-ingestion.md), [0004](0004-player-lifecycle-and-device-lookups.md), and [0007](0007-active-event-api-scope.md). Their other decisions remain in effect.

## Context

We allowed devices to upload to an older event while another was running.
We don't need that behavior; uploads should follow the operator's current
selection like the rest of the API.

## Decision

Accept match uploads only for the active event. Hold that selection through
commit, and reject references to players, teams, or saved matches from another
event. Event management is the only operation that selects an event explicitly.

Keep batch atomicity, first-report-wins behavior, and the existing scoring rules.

## Alternatives considered

Keeping explicit event targeting would let devices finish uploading old events,
but adds an exception we don't need.

## Consequences

Clients use one upload URL. Pending results from a previous event are rejected
unless an operator selects that event again. Devices should finish uploading
before the operator switches events.

## References

- [Routes and OpenAPI](../../src/routes/quickdraw.rs)
- [Device contract](../../proto/device_api.proto)
