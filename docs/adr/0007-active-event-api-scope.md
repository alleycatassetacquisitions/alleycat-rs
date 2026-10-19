# 0007: Use the active event for routine API operations

- Status: Partially superseded by [0009](0009-active-event-match-uploads.md) for the explicit-event exception for match uploads. Other decisions remain in effect.
- Date: 2026-10-18

## Context

Operators select the event they are running. Routine requests should then use
that selection, without making clients send the event ID again on every call.
Player registration, lookups, and editing already work this way.

Some management routes were built with explicit event IDs. When adding
affiliations, we copied that pattern and lost the intended convention. We need
one clear rule for future work, including which existing routes are awaiting
repair rather than examples to follow.

## Decision

Routine reads and writes for an event use the server's active event. This
includes players, teams, affiliation categories and choices, scoring controls,
and routine match browsing. Clients supply resource IDs where needed, but do
not select the event through a path, query parameter, or request-body override.
Resource IDs from another event must never bypass the active-event boundary.
Responses can still include event IDs to identify the data they describe.

Resolve the selection once for an operation. Writes hold the active-event
selection lock through commit, so a concurrent event switch cannot redirect an
edit or finish while that edit is still being saved. Reads must use the same
selection for their parent checks and returned data. Without an active event,
never fall back to another event or return data from all events. OpenAPI records
each endpoint's response behavior.

There are deliberate exceptions:

- Managing events themselves, including listing, creating, editing, and selecting
  an event, must work independently of the active selection.
- Match uploads target the event captured when the match happened. A delayed
  upload must not land in whichever event is active when it arrives. This keeps
  the explicit-event ingestion decision in ADR 0002.

New exceptions need an explicit architecture decision. An existing URL with an
event ID is not, by itself, a reason to copy that pattern.

## Alternatives considered

- Explicit event IDs everywhere would support managing several events at once,
  but make clients repeat a selection the server already owns and allow routine
  operations to diverge from the event the operator selected.
- Supporting both implicit and explicit event selection on routine routes would
  leave two ways to choose the target and preserve the ambiguity we are removing.
- Using the active event for delayed uploads would misattribute or reject valid
  matches after an event switch.

## Consequences

Clients select an event and then use a consistent set of routine operations.
Managing another event through those operations requires changing the selection.
Clients should refresh event-specific views when that selection changes.

This change applies the rule to affiliation categories and affiliations. Player
operations already follow it. Existing team routes, scoring controls, the
delayed-score setting, and match listing remain unchanged in this change and
need a follow-up repair. They are unfinished work, not additional exceptions.
Match ingestion and event management retain their explicit targeting.

This decision extends ADR 0004's active-event behavior to routine operations;
it does not replace the affiliation model or the delayed-upload decision.

## Implementation follow-up

As of 2026-10-18, the remaining routine routes also use the active event. The
follow-up covers teams, scoring controls, the delayed-score setting, and match
listing. [ADR 0009](0009-active-event-match-uploads.md) also brings match uploads
under the active selection, leaving event management as the only exception. See
the route annotations below for the current contracts.

## References

- [Player lifecycle and event-selection locking](0004-player-lifecycle-and-device-lookups.md)
- [Explicit event targeting for device ingestion](0002-quickdraw-device-ingestion.md)
- [Event affiliations](0006-event-affiliations.md)
- [Affiliation routes and OpenAPI](../../src/routes/affiliations.rs)
- [Active-event helpers](../../src/events.rs)
- [Team routes and OpenAPI](../../src/routes/teams.rs)
- [Scoring controls and OpenAPI](../../src/routes/scoring.rs)
- [Game state and delayed-score control](../../src/routes/game_state.rs)
- [Match listing and ingestion](../../src/routes/quickdraw.rs)
