# Device player lookups

- Status: Accepted

Give devices a separate response and database query that exclude email and other
staff-only information. Devices need player identity and display information,
not registration contact details. Include affiliation IDs for stable identity
and names to avoid a second management lookup; let operators filter categories
so devices receive only what they need.

Apply current-PDN display constraints when constructing the response. Limited
memory and ASCII displays should not destroy stored Unicode names that other
clients or future devices can use. Server-side conversion also guarantees safe
output when a client bypasses UI guidance. Use the existing transliteration
library rather than maintain custom language tables, and one current-device
profile rather than negotiate capabilities for variants we do not yet have.

Raw UUID bytes reduce size without the collision risk of shortened hashes.
Transliterated names can become empty or identical, so identity must still use
UUIDs. Per-string limits do not bound the whole response or firmware allocation;
affiliation counts and device storage remain separate constraints. Future devices
can use a different profile while existing firmware retains its guarantees.

## References

- [Device contract](../../proto/device_api.proto)
- [PDN display conversion](../../src/routes/helpers/device_display.rs)
- [Player lookups](../../src/routes/players/get.rs)
- [Affiliation management](../../src/routes/affiliations.rs)
- [Event affiliations](event-affiliations.md)
