# alleycat-rs

## Running the Server

### requirements

- Docker Desktop, or Docker Engine with Docker Compose
- this repository checked out on the machine
- `bunyan` is optional and only used for prettier logs

### first-time setup

- copy `.env.example` to `.env` and set a local `POSTGRES_PASSWORD`
- start the UI, API, and Postgres with `scripts/start_local_instance.sh`
- open the UI at http://localhost:3000
- test the app with `curl -i http://localhost:8000/health_check`
- view API logs with `scripts/view_logs.sh`
- view UI logs with `docker compose logs --no-log-prefix -f web`

Use a URL-safe database password for local Docker because the migration
container passes it through a Postgres connection URL.

### routine commands

- start the UI, API, and Postgres with `scripts/start_local_instance.sh`
- to rebuild after code changes run `docker compose build && scripts/start_local_instance.sh`
- stop the containers without deleting them with `docker compose stop`
- back up app logs with `scripts/backup_logs.sh`
- back up the database with `scripts/backup_db.sh`
- remove the containers, network, and database volume with `scripts/delete_local_instance.sh`

Compose runs SQLx migrations before starting the API, then starts the Remix UI
on port 3000. The UI reaches the API over Compose's private network at
`http://app:8000`. Docker keeps recent container logs on the host using the
`local` log driver with rotation while the containers exist. Postgres data is
stored in a named Docker volume; `docker compose down --volumes` deletes it.

Backups are written under `backups/`, which is ignored by git.
`scripts/delete_local_instance.sh` runs both backup scripts before deleting the
local database volume.

If application code, migrations, or Docker files change, rebuild the images
before restarting:

```bash
docker compose build
scripts/start_local_instance.sh
```

## development requirements

- rust
- sqlx-cli, installed with `cargo install sqlx-cli`
- docker

## development setup

- run `scripts/init_db.sh`
- `cargo run`
- in another terminal, run `cd web && npm run dev`; the UI defaults to the API
  at `http://localhost:8000`

## testing setup

- run `scripts/prepare_sqlx.sh`
- `cargo test`
