# 0004: Player lifecycle and device lookups

- Status: Accepted
- Date: 2026-10-15

## Context

Registration operators need to correct player details, change teams, and mark
registrations inactive without deleting participants referenced by match history.
Devices need PIN lookup data without receiving the email used at registration.
PINs are unique only within an event and can be reused in other events.

## Decision

Keep the player UUID, event, assigned PIN, and creation time immutable. Permit
updates to name, email, current team, and an active/inactive flag. Nullable
email and membership may be cleared. Team membership stays within the event.
Changes apply only to registrations in the active event; updates hold the active
selection lock until commit so an event switch cannot redirect an edit.

Deactivation is reversible status, not deletion or a scoring decision. Inactive
players remain listed and discoverable, retain their names and PINs, and can be
reactivated. Device responses expose the flag for clients to honor. Neither
deactivation nor changing teams rewrites match snapshots or rejects previously
completed matches uploaded later. This flag alone does not prove whether a
player was active at match time; ingestion continues to trust device reports.

Staff JSON responses include all registration fields and the current team's
name. Device lookup has a separate protobuf response with no email field and
queries only device-visible data. Both UUID and PIN lookups use the active event.
Devices receive the event UUID alongside the PIN so match uploads can retain
their original event even after the active selection changes.

## Alternatives considered

- Hard deletion would break participant references or require erasing history.
- Using current membership/status to invalidate historical matches cannot account
  for delayed device uploads and would change previously earned credit.
- Global PIN lookup would be ambiguous across events.
- Sharing the staff response with devices would unnecessarily disclose email and
  make future response changes easier to expose accidentally.

## Consequences

Inactive registrations continue to consume a PIN and reserve their name in that
event. Staff can correct or reactivate the original record. Enforcing eligibility
at match time would require a separate policy and temporal status information.
Corp/faction and role categories remain a separate design task.

## References

- [Player routes and OpenAPI](../../src/routes/players/)
- [Device protobuf contract](../../proto/device_api.proto)
- [Player schema](../database/schema/public.players.md)
- [Match-time team snapshots](0002-quickdraw-device-ingestion.md)
