# Quickdraw raw reports and diagnostics

- Status: Accepted

Retain eligible raw submissions, including duplicates, failures, and rejected
entries, to investigate device, communication, and upload problems or conflicting
reports. Completed results alone lose that evidence. Keep raw evidence even for
cross-event ID conflicts that prevent saving completed results.

Store known submitted fields in queryable columns, preserving values that completed
result validation rejects. This supports SQL analysis without requiring binary
payload decoding or storing the same data twice. It also means new protobuf fields
need corresponding storage and mapping changes; unknown fields are not retained.

Compare gameplay outcomes after normalizing participant order, rather than compare
entire reports: expected differences in timestamps or other metadata should not
look like disagreements. Diagnostics must not reconcile outcomes or rewrite saved
scores, which would move game rules into the server.

Storage grows with every submission, with no automatic retention policy. Reporter
counts are evidence rather than proof of missing uploads: optional MAC addresses
and delayed second reports limit what those counts can establish.

## References

- [Device contract](../../proto/device_api.proto)
- [Upload and match listing](../../src/routes/quickdraw.rs)
- [Raw report endpoints](../../src/routes/quickdraw_reports.rs)
- [Raw report schema](../database/schema/public.quickdraw_matches_raw.md)
- [Scoreboard decision](player-scoreboard.md)
