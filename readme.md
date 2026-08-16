# Alleycat server

Alleycat combines a Rust HTTP API, a server-rendered Remix UI, and PostgreSQL.
It registers and lists players, ingests protobuf device crash logs, and
displays players and logs in the UI.

## Run the complete stack locally

Run the deployment-shaped local stack with Docker Compose:

```bash
test -f .env || cp .env.example .env
```

Set `POSTGRES_PASSWORD` in `.env` to a URL-safe local password, then run:

```bash
scripts/start_local_instance.sh
scripts/seed_db.sh
```

Then open:

- UI placeholder home: <http://localhost:3000>
- API health check: <http://localhost:8000/health_check>
- Players: <http://localhost:3000/players>
- Device logs: <http://localhost:3000/device-logs>

Rebuild after source changes:

```bash
docker compose build
scripts/start_local_instance.sh
```

`docker compose stop` preserves its data. Read the
[operations guide](docs/operations.md) before deleting containers or databases.

## Documentation

The [documentation map](docs/README.md) routes maintainers, operators, and
device developers to current guides and separates them from planning material.

## Repository map

| Path | Responsibility |
| --- | --- |
| [`src/`](src/) | Rust API, domain validation, configuration, and telemetry |
| [`web/`](web/) | Server-rendered Remix UI and its API client |
| [`proto/`](proto/) | Device-facing Protocol Buffer contract |
| [`migrations/`](migrations/) | Ordered PostgreSQL schema migrations |
| [`configuration/`](configuration/) | Layered API configuration for local and production use |
| [`scripts/`](scripts/) | Development and operations helpers |
| [`tests/api/`](tests/api/) | Black-box API tests against temporary PostgreSQL databases |
| [`spec.yaml`](spec.yaml) | Desired DigitalOcean App Platform topology |

The project is licensed under [AGPL-3.0-only](LICENSE).
