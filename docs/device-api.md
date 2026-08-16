# Device crash-report API

Devices send one `alleycat.device.WriteDeviceLogRequest` Protocol Buffer
message over HTTP, not gRPC. The unversioned schema is
[`proto/device_api.proto`](../proto/device_api.proto).

## Endpoint

Configure the base URL for the environment:

| Environment | API base |
| --- | --- |
| Same machine as server | `http://localhost:8000` |
| Device on a trusted LAN | `http://<server-lan-ip>:8000` |
| Shared DigitalOcean server | `https://<app-origin>/api` |

The maintainer supplies the shared origin. The endpoint is unauthenticated.
For shared HTTPS, the device clock may need to be accurate enough to validate
the server certificate.

For LAN tests, Compose exposes host port `8000`; native `cargo run` listens only
on loopback. Use a trusted, firewalled network.

Send each report to:

```http
POST <API_BASE>/device-logs
Content-Type: application/protobuf
```

Use the exact path without a trailing slash.

## Request

Generate a client binding from `proto/device_api.proto`, populate
`alleycat.device.WriteDeviceLogRequest`, serialize it, and send the serialized
bytes as the entire HTTP body.

| Field | Tag | Required by server | Protobuf type | Meaning |
| --- | ---: | :---: | --- | --- |
| `device_mac` | 1 | Yes | `string` | Canonical 48-bit form: `aa:bb:cc:dd:ee:ff` |
| `crash_number` | 2 | Yes | `uint64` | Durable, device-assigned report number; maximum `9,223,372,036,854,775,807` |
| `uptime_ms` | 3 | Yes | `uint64` | Milliseconds since boot; same maximum |
| `software_version` | 4 | No | `string` | Firmware or build identifier |
| `reset_reason` | 5 | Yes | `uint32` | Raw platform reset-reason code; maximum `2,147,483,647` |
| `program_counter` | 6 | No | `fixed32` | Instruction address at the exception |
| `exception_cause` | 7 | No | `uint32` | Raw platform exception-cause code |
| `task_name` | 8 | No | `string` | FreeRTOS task active at the exception |

The `.proto` fields are declared `optional` so the server can distinguish a
missing scalar from zero. The four fields marked required must be present;
zero is valid for numeric fields.

With a system `protoc` installed, this creates an exact smoke-test payload from
the repository root:

```bash
protoc \
  --proto_path=proto \
  --encode=alleycat.device.WriteDeviceLogRequest \
  proto/device_api.proto > report.pb <<'EOF'
device_mac: "aa:bb:cc:dd:ee:ff"
crash_number: 42
uptime_ms: 123456
software_version: "0.1.0"
reset_reason: 1
program_counter: 1074270772
exception_cause: 6
task_name: "main"
EOF
```

Post the payload with:

```bash
export ALLEYCAT_API_BASE=http://localhost:8000
curl -i \
  -H 'Content-Type: application/protobuf' \
  --data-binary @report.pb \
  "$ALLEYCAT_API_BASE/device-logs"
```

## Responses

| Status | Meaning | Device behavior |
| ---: | --- | --- |
| `204 No Content` | Report acknowledged; response body is empty | Remove the queued report |
| `400 Bad Request` | Malformed protobuf, missing required field, invalid MAC, or out-of-range value | Treat as a firmware/report defect; do not retry forever |
| `413 Payload Too Large` | Serialized HTTP body exceeds `262,144` bytes (256 KiB) | Treat as a firmware/report defect |
| `415 Unsupported Media Type` | Missing or incorrect `Content-Type` | Fix the request headers |
| `500 Internal Server Error` | The server could not persist the report | Retry with backoff |

Error bodies are plain diagnostic text and are not a stable machine-readable
contract. Branch on the HTTP status.

## Retry and report numbering

The server identifies a report by `device_mac` plus `crash_number`.

- Persist a counter so a crash number is never reused for the same MAC.
- Retain the exact payload until the server returns `204`.
- Retry connection failures and `5xx` responses with exponential backoff and
  jitter, using the same MAC, crash number, and payload.
- After a `4xx`, stop automatic retries and retain the report for debugging.
  Changing fields while reusing its crash number is not a correction.
- Reports may arrive in any order.

The server also returns `204` for an existing MAC/crash-number pair but does
not replace its data. Reusing the counter can therefore make an ignored report
look stored.

The server timestamps receipt; the payload has no crash-time field.

## Integration check

After a `204`, a human can check the operator page:

- Local: <http://localhost:3000/device-logs>
- Shared: `https://<app-origin>/device-logs`

That page and the JSON `GET /device-logs` endpoint are operator interfaces.
Firmware must not depend on either one.

## Compatibility rules

Until the protocol has an explicit version:

- Never change or reuse an existing field number.
- Never change an existing field's wire type or meaning.
- Add optional fields with new numbers.
- Reserve the number and name of a removed field.
- Do not make a new field server-required without a version transition and a
  coordinated firmware rollout.

Open coordination decisions are crash-counter reset/wrap behavior, offline
queue limits, reset and exception code mappings, and stable identity when a MAC
can change.
