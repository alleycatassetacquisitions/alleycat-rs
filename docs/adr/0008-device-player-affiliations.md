# 0008: Include affiliations in device player lookups

- Status: Accepted
- Date: 2026-10-18
- Supersedes: [0006](0006-event-affiliations.md) only for keeping affiliations out of device responses.

## Context

We initially kept affiliations in the management JSON API because devices did
not need them. A consuming client now needs these assignments in the protobuf
player lookup too.

## Decision

Include current category and affiliation IDs and names in the device player
lookup. IDs give clients stable identities; names let them display the choices
without another management API request. Read them together with the player's
other device-visible details in the active event.

Keep the device message and query separate from the management response. Email
stays out of both the device message and its database query.

Extend the protobuf message with a new field, preserving the existing field
numbers and meanings. Older protobuf clients can ignore the addition; clients
that need affiliations must regenerate their bindings from the updated proto.
The proto comments remain the reference for fields and response behavior.

## Alternatives considered

- Requiring a management JSON lookup would add a second request and expose
  information, such as email, that devices do not need.
- Sending IDs alone would keep the message smaller but require another way for
  clients to obtain names for display.
- Sending names alone would make renaming look like a change of membership.

## Consequences

Device lookups grow with the number of assigned categories. The assignments
describe current membership and names; clients must refresh the lookup to see
changes. This does not introduce permissions, historical assignment snapshots,
or changes to match uploads and scoring.

## References

- [Event affiliation model](0006-event-affiliations.md)
- [Player lifecycle and device privacy](0004-player-lifecycle-and-device-lookups.md)
- [Active-event API scope](0007-active-event-api-scope.md)
- [Device protobuf contract](../../proto/device_api.proto)
- [Player lookup routes and OpenAPI](../../src/routes/players/get.rs)
