# Alleycat server

Alleycat is a Rust HTTP API backed by PostgreSQL. This guide runs the API and
database together on one machine with Docker Compose.

> This deployment publishes an unauthenticated API on port `8000`. Use it only
> on a trusted network; do not expose it directly to the internet.

## Requirements

- Git
- [Docker with Docker Compose](https://docs.docker.com/get-started/get-docker/)
- [Rustup](https://rustup.rs/), which provides Cargo

You do not need Rust experience to deploy Alleycat. Rust is required to run the
repository's task interface. The repository selects Rust `1.95.0`
automatically; the first Cargo command may download that toolchain and compile
the task program.

## Deploy locally

After cloning the repository, run these commands from its root.

Create the local configuration:

```bash
cp .env.example .env
```

Replace `POSTGRES_PASSWORD` in `.env` with a URL-safe local password, then
check the tools and configuration:

```bash
cargo xtask stack doctor
```

Build and start the containers:

```bash
docker compose build
cargo xtask stack start
```

Confirm that the API is running:

```bash
curl -i http://localhost:8000/health_check
```

A healthy API returns `200 OK`. If you changed `APP_PORT`, use that port in the
health-check URL. To add repeatable sample players and device
logs:

```bash
scripts/seed_db.sh
```

## Routine commands

```bash
# Follow API logs.
scripts/view_logs.sh

# Stop the containers while preserving the database.
docker compose stop

# Start them again and apply pending migrations.
cargo xtask stack start
```

Compose uses built images rather than live source files. After pulling or
changing application code, rebuild before restarting:

```bash
docker compose build
cargo xtask stack start
```

PostgreSQL data remains in a Docker volume when the containers stop.
`scripts/delete_local_instance.sh` asks for confirmation, backs up logs and the
database, and then deletes the local containers and database volume.

## Native development

For API development outside Compose, initialize the standalone development
database and run the server:

```bash
scripts/init_db.sh
cargo run
```

Run the test suite with:

```bash
cargo test
```
