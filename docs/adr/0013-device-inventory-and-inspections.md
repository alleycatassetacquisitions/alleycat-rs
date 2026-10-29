# 0013: Keep physical devices and their inspection history across events

- Status: Accepted
- Date: 2026-10-28

## Context

Operators need a device's last known condition and a history of failures and
repairs. Devices exist between events, and crash reports cannot establish their
physical condition.

## Decision

Keep permanent device identities and append-only inspection snapshots, independent
of the active event. This extends the exceptions in [ADR 0007](0007-active-event-api-scope.md).

Each snapshot records observations and firmware at check time. Unchecked components
stay unknown; values never carry forward. Order history by check time so delayed
uploads cannot replace newer observations. Client-assigned UUIDs make retries safe
without overwriting accepted records. Creation uses protobuf; management reads use JSON. Device reports register unknown
devices and save their inspection atomically, so firmware need not track registration.

## Alternatives considered

Overwriting current condition loses history. Recording only changes can make old
observations look newly verified. Event-owned inventory would fragment that history
and prevent between-event checks.

## Consequences

History supports later failure analysis, but a latest inspection does not certify
readiness. Inventory shares the application database and is deleted by its reset.
Independent backups and tested recovery are still needed; export/restore and the
operator interface remain follow-ups.

## References

- [Issue 56](https://github.com/alleycatassetacquisitions/alleycat-rs/issues/56)
- [Device-specific schemas](0014-device-specific-inspection-schemas.md)
- [Database backup and recovery](../database-backups.md)
- [Protobuf contract](../../proto/device_api.proto) · [HTTP contracts](../../src/routes/devices/)
- [Device schema](../database/schema/public.devices.md) · [Inspection schema](../database/schema/public.device_inspections.md)
