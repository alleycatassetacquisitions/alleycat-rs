# Quickdraw ingestion

- Status: Accepted

Accept valid completed matches from partially invalid batches because one
opponent's bad registration data should not block unrelated offline results.
Report per-entry failures so firmware can distinguish its own bad data from an
opponent's. Reporting the first error is sufficient, though correction may reveal
another. Rejecting whole batches would force every device to split and retry them.

Keep the first accepted completed result, including ties, so retries cannot
replace earned credit. Rejected entries and failures leave the ID available for
a completed result. Trust reported outcomes and match-time team snapshots: current
membership cannot reconstruct an offline match, and maintaining membership history
would add state we do not need. This accepts that the server cannot independently
prove those snapshots. Protobuf keeps one wire format across device APIs.

Use savepoints to isolate expected reference failures while committing valid
results together. Separate transactions per match could leave results saved despite
an error response. Hold the event scoring lock and read settings in a subsequent
statement so an upload that waited for a setting change sees the committed change.
This serializes uploads with scoring controls; finer locking is warranted only
if measured throughput requires it.

[Raw reports](quickdraw-raw-reports.md) preserve diagnostic evidence separately;
[scoring](quickdraw-scoring.md) owns eligibility and
[event scope](event-api-scope.md) owns event selection.

## References

- [Device contract](../../proto/device_api.proto)
- [Upload contract and implementation](../../src/routes/quickdraw.rs)
- [Ingestion tests](../../tests/api/quickdraw.rs)
- [Quickdraw scoring](quickdraw-scoring.md)
