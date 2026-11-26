# Device inventory and inspections

- Status: Accepted

Keep permanent device identities and append-only inspection snapshots across
events because operators need a continuous history of failures and repairs.
Crash reports cannot establish physical condition. Register unknown devices with
their report so firmware need not track registration separately.

Record complete observations without carrying unchecked values forward; otherwise
old observations can look newly verified. Order history by check time so delayed
uploads do not replace newer observations. A latest inspection describes what was
checked, not a certification of readiness.

Use typed checks for each device kind alongside shared inspection metadata.
A universal PDN checklist would make irrelevant components on future devices look
unchecked; arbitrary field maps would weaken firmware contracts. Separate APIs
would duplicate the shared workflow. Typed schemas cost a migration and code
changes per new kind, but let devices differ without changing existing checks.
Share component definitions only when their meaning and validation match.

## References

- [Protobuf contract](../../proto/device_api.proto)
- [Device HTTP contracts](../../src/routes/devices/)
- [SQL schema](../database/schema.sql)
- [Database backup and recovery](../database-backups.md)
