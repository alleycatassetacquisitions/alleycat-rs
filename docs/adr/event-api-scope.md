# Event API scope

- Status: Accepted

Use the server's active event for routine event operations so clients consistently
follow the operator's selection. Explicit event IDs, or allowing both selection
methods, would let routine requests diverge from that choice. Resource IDs must
not bypass this boundary, and an absent selection must not imply another event
or all events.

Resolve the selection once per operation and hold it through write commits so an
event switch cannot redirect work in progress. Switching may have to wait for
those writes. Clients must refresh event-specific views after a switch.

Event management is independent because operators must be able to choose an event.
[Device inventory](device-inventory-and-inspections.md) is independent because
physical devices exist between events. These are deliberate exceptions; new
exceptions need an agreed update here.

Match uploads follow the active selection too: uploading for older events while
another runs is not a supported requirement. Devices should finish uploading
before a switch; otherwise an operator must reselect the old event to accept its
pending results.

## References

- [Active-event helpers](../../src/events.rs)
- [Event management](../../src/routes/events.rs)
- [Quickdraw upload contract](../../src/routes/quickdraw.rs)
- [Player routes](../../src/routes/players/)
- [Device routes](../../src/routes/devices/)
