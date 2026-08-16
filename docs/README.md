# Documentation map

Start with the document for your task:

| Need | Document |
| --- | --- |
| Resume work after time away | [Developer guide](development.md) |
| Understand how the pieces fit | [Architecture](architecture.md) |
| Run or deploy the whole application | [Operations and deployment](operations.md) |
| Decide whether a script is safe to run | [Script reference](scripts.md) |
| Integrate device firmware with the server | [Device API](device-api.md) |
| Use the current operator UI | [Operator guide](operator-guide.md) |
| See the product backlog and porting ideas | [Stories](stories/README.md) |

## What is authoritative?

All entries above except Stories describe the current repository. When
documentation drifts, code and tests are authoritative; update the docs in the
same change.

Planning and historical material:

- `docs/stories/` may describe unimplemented behavior.
- [`api-contract.md`](api-contract.md) documents the earlier Hono/Bun server,
  not the Rust API.
- [`archive/schema.ts`](archive/schema.ts) is an inactive schema snapshot.

## Keep the docs current

When a boundary changes, review:

| Change | Also review |
| --- | --- |
| Route, request, or response | `architecture.md`; `device-api.md` when a device consumes it |
| Operator page or displayed field | `operator-guide.md` |
| Protocol Buffer field | `proto/device_api.proto` and `device-api.md` |
| Script behavior or environment variable | `scripts.md` and the workflow that calls it |
| Compose service or port | `architecture.md`, `development.md`, and `operations.md` |
| DigitalOcean component, ingress rule, or binding | `architecture.md` and `operations.md` |
| Table, view, sequence, routine, or custom type | migration docs and the explicit inventory in `scripts/reset_remote_db.sh` |
| Native toolchain or test command | `development.md` |

Link to source instead of copying implementation details. External contracts
should omit internal database and framework details.
