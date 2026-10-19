# 0010: Accept valid matches from partially invalid batches

- Status: Accepted
- Date: 2026-10-18
- Supersedes: Batch atomicity and repeated-ID handling in [0002](0002-quickdraw-device-ingestion.md) and batch atomicity in [0009](0009-active-event-match-uploads.md). Their other decisions remain in effect.

## Context

A player might finish several matches before their device can report them to
the server. If just one opponent had bad registration data, rejecting the whole
upload would hold up all of those results. Retrying the same batch would keep
failing.

The device also needs enough information to decide what to do next. It knows
which participant it was in each match, so identifying the bad fields lets it
check whether the problem involves its own saved data or an opponent's.

## Decision

Save the valid matches and tell the device which ones failed. For each rejected
match, return the first error we find, including the affected fields and a reason
the firmware can read directly. Any entry not listed as rejected was saved or
was already stored.

Keep the active-event and scoring locks until the valid matches are saved. Each
match we try to insert gets a database savepoint, so a bad player or team reference
can be rolled back without losing the other matches.

Some problems still stop the whole upload: an unreadable or invalid request, no
active event, a match ID already belonging to another event, or an unexpected
database failure. In those cases, save no new matches. Check whether IDs belong
to another event even when those matches also have invalid fields.

The first accepted report still wins. Retrying it never changes the saved result.
If an ID appears more than once in a batch, use the first occurrence we can accept;
rejecting an earlier occurrence does not prevent a later valid one from being
saved. Identify rejected entries by their position in the request as well as
their submitted ID, since IDs can be repeated or malformed.

The protobuf comments and OpenAPI annotations below define the message fields
and HTTP responses.

## Alternatives considered

- We could keep rejecting whole batches and have devices split them up to find
  the bad matches. That keeps the server simpler, but every device would need
  recovery logic and more upload attempts.
- We could save each match in a separate transaction. If the server then failed
  partway through, some matches would already be saved despite the error response.
  Savepoints let us handle individual bad matches while still saving the accepted
  results together at the end.
- We could report every bad field in each match. That would take more validation
  work; reporting the first problem is enough for this change.

## Consequences

One opponent's bad data no longer holds up a player's other results. Devices can
remove accepted matches from their queues and keep rejected ones for correction
or review. Firmware needs to understand the new response before using it to clear
the queue. If a response is lost, retrying with the same match IDs is still safe.

Savepoints add some database work for each match, within the existing batch limit.
Fixing the first error may reveal another one on the next attempt. The device
decides how to act on the reported fields, using its own saved data. If the error
involves its own identity in an older match, it should check its current
registration before sending the player back to base.

## References

- [Device request and response contract](../../proto/device_api.proto)
- [Upload routes and OpenAPI](../../src/routes/quickdraw.rs)
- [Ingestion tests](../../tests/api/quickdraw.rs)
