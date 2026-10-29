# 0014: Give each device kind its own inspection schema

- Status: Accepted
- Date: 2026-10-28
- Extends: [0013](0013-device-inventory-and-inspections.md)

## Context

Only PDNs are supported today. Future devices may have overlapping components but
different inspection schemas. A universal PDN checklist would make irrelevant
components look merely unchecked.

## Decision

Keep shared inspection metadata separate from device-specific checks. Use a protobuf
`oneof` and a separate component table for each supported kind. JSON reads identify
the payload's kind. Accept only known payloads matching the registered device.

Save metadata and checks atomically; database constraints enforce matching kinds.
Add new schemas when their requirements are known. Share component definitions only
when their meaning and validation match.

## Alternatives considered

A universal schema accumulates irrelevant fields. Arbitrary JSON or field maps
weaken firmware contracts and require extra validation. Separate APIs would duplicate
the shared inspection workflow.

## Consequences

Future kinds can differ without changing PDN checks. Reads require a join and writes
a transaction. Each new kind still requires code, a migration, and documentation.

## References

- [Protobuf contract](../../proto/device_api.proto) · [HTTP contracts](../../src/routes/devices/)
- [Shared inspection schema](../database/schema/public.device_inspections.md)
- [PDN checks schema](../database/schema/public.pdn_inspection_checks.md)
