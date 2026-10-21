# 0004: Player lifecycle and device lookups

- Status: Partially superseded by [0009](0009-active-event-match-uploads.md) for accepting uploads after an event switch and [0012](0012-current-pdn-constraints.md) for device response fields. Other decisions remain in effect.
- Date: 2026-10-15

## Context

Operators need to fix player details, assign players to teams, and mark a
player inactive. Deleting a player would cause problems for matches they've
already played, so we need a way to keep that history intact.

Devices also need to find players by PIN, but they don't need the email address
used at registration. Since two events can assign the same PIN, a lookup needs
to know which event we're running.

## Decision

Operators can change a player's name, email, team, and active status. They can
also clear an email address or remove a player from a team. A player can only
join teams in their own event. Their UUID, event, assigned PIN, and registration
time stay fixed so edits don't change which registration we're referring to.

Edits apply to players in the active event. While saving an edit, we lock the
active event selection so switching events can't redirect an edit already in
progress.

An inactive player keeps their name and PIN, still appears in lookups and lists,
and can be reactivated. Devices receive that status so they can act on it.
Changing status or teams doesn't rewrite past matches or reject completed
matches that arrive later. Today's status can't tell us whether someone was
active when a match happened, so we continue to trust the device's match report.

The JSON responses used by registration staff include the player's full details
and current team name. Devices get a separate protobuf message with no email
field; that lookup doesn't even read email from the database.

Both UUID and PIN lookups use the active event. The device response includes the
event UUID so a device can save it with a match and upload to the correct event
later, even if the server has since switched events.

## Alternatives considered

- Deleting players would leave matches referring to missing participants or
  require us to erase their history too.
- Using a player's current team or status to decide whether an old match counts
  would mishandle delayed uploads and could change points already earned.
- Looking up a PIN across all events could return more than one player.
- Sending devices the staff response would give them email addresses they don't
  need. A separate message also keeps future staff-only fields out of device
  responses unless we deliberately add them.

## Consequences

Inactive players still use a PIN and reserve their name in that event. Staff can
correct or reactivate the existing registration instead of creating another one.

If we later want the server to enforce whether a player was allowed to play at
match time, we'll need rules for that and a history of status changes. The
active flag alone isn't enough. Corp/faction and role categories are also left
for a separate design discussion.

## References

- [Player routes and OpenAPI](../../src/routes/players/)
- [Device protobuf contract](../../proto/device_api.proto)
- [Player schema](../database/schema/public.players.md)
- [Match-time team snapshots](0002-quickdraw-device-ingestion.md)
