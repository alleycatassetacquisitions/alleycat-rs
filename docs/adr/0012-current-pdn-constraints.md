# 0012: Keep current PDN constraints at the device response boundary

- Status: Accepted
- Date: 2026-10-20
- Supersedes: [0004](0004-player-lifecycle-and-device-lookups.md) for device response fields, and [0008](0008-device-player-affiliations.md) for unconditional affiliation exposure and its additive-only response change.

## Context

Current PDNs have limited memory and display only printable ASCII. UUID text,
fields the device does not use, and every assigned affiliation make the player
lookup larger than it needs to be. Fixed firmware buffers can also occupy more
memory than the encoded protobuf message.

These are constraints of the current device, not reasons to discard Unicode
names from the database or restrict other clients. Future devices may support
more text and different displays.

## Decision

Keep a separate, compact protobuf response for player lookups by PIN. Send full
UUIDs as raw bytes rather than their hexadecimal text. Remove the response's
PIN, event ID, creation time, and active status. The path still supplies the PIN,
and the server still resolves it within the active event. Inactive registrations
remain discoverable; this change does not add a rule preventing them from playing.

Let operators decide which affiliation categories devices need. Categories have
a `send_to_devices` setting that defaults to false when created. The device query
includes only assignments from enabled categories. Staff JSON responses retain
all assignments. Changing the setting affects subsequent device lookups, not
membership, permissions, scoring, or already cached responses. The new migration
also leaves existing categories disabled until an operator enables them.

Apply a single current-PDN display profile when constructing device display
strings. Use `deunicode` for transliteration, replace unknown and remaining
non-printable characters with `?`, and truncate the converted result to 255 ASCII
characters. That fits in a 256-byte C buffer with a terminator. Protobuf strings
carry only their actual contents; the server does not pad them or send a terminator.
The profile covers player, team, category, and affiliation names, and should be
used for future display text on this device path.

Keep stored text and existing input validation unchanged. UIs can guide users
toward names that display well on PDNs, but the server must guarantee safe device
output even when another client writes Unicode text. All display conversion lives
in one small helper; there are no transliteration tables or device-specific text
columns in the database. A future device can use a different response profile
without losing the original names. Existing firmware must continue to receive
the limits it was built for; relaxing them requires an explicit contract change
or a separate device response.

Keep exact field definitions and conversion rules in the protobuf comments, and
category management behavior in OpenAPI. This decision applies to the device
player response. Match uploads and staff JSON continue to use their existing
UUID representations.

## Alternatives considered

- Shortening UUIDs with a hash would introduce collision risk. Raw bytes preserve
  the entire UUID without a lookup table.
- Restricting or truncating stored names for the device would lose information
  that other clients and future devices can use.
- Trusting UI validation alone would let other clients send text the device
  cannot display. Output conversion provides the guarantee at the consuming boundary.
- Maintaining custom transliteration rules would add language-specific code;
  `deunicode` already supplies the conversion and is present in our dependency graph.
- Sending every affiliation and filtering on the device would still use network
  bandwidth and decoding space for assignments the device does not need.
- A negotiated capability system would add machinery for device variants we do
  not yet have. A single response profile is sufficient for the current PDN.

## Consequences

This is a breaking protobuf contract change. Deploy the server and regenerated
firmware bindings together. Removed field numbers and names are reserved. Keeping
the remaining UUID field numbers does not make their new binary contents compatible
with clients expecting UTF-8 UUID strings. Devices converting these UUIDs back to
text for existing match uploads must preserve their byte order.

Transliteration is approximate and can expand text before truncation. Names can
become empty or identical after conversion, so devices must use UUIDs for identity.
The display profile bounds each string, not the entire response: affiliation count
still depends on assignments in enabled categories. Operators must enable only
categories devices need; a fixed firmware array needs a separate agreed count limit.

Device storage layout is still the firmware's responsibility. It can keep only
needed values and share category or affiliation definitions across cached players.
Neither a shorter wire message nor a string limit determines allocated RAM on its own.

## References

- [Device protobuf contract](../../proto/device_api.proto)
- [PDN display conversion](../../src/routes/helpers/device_display.rs)
- [Device lookup and OpenAPI](../../src/routes/players/get.rs)
- [Category management and OpenAPI](../../src/routes/affiliations.rs)
- [Category schema](../database/schema/public.affiliation_categories.md)
- [Category visibility migration](../../migrations/20261020000000_add_category_device_visibility.sql)
