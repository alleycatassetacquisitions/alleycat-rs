# Operations and deployment

Use this runbook for the complete local stack and shared DigitalOcean
environment. For host-native coding and tests, use the
[Developer guide](development.md#native-development).

## Production-like local stack

Docker Compose runs PostgreSQL 18, a one-shot SQLx migration job, the release
Rust API image, and the production-mode Remix server.

### First setup

Requirements are Docker and Docker Compose. From the repository root:

```bash
cp .env.example .env
```

Set all three values in `.env`:

```dotenv
POSTGRES_USER=app
POSTGRES_PASSWORD=<URL-safe local password>
POSTGRES_DB=alleycat
```

Use a local-only, URL-safe password because Compose interpolates it directly
into the migration job's PostgreSQL URL. `.env` is gitignored.

Build and start:

```bash
docker compose build
scripts/start_local_instance.sh
```

The start script removes the previous one-shot `migrate` container so pending
migrations run again, then starts the project in the background. Compose waits
for PostgreSQL health and successful migration before it starts the API. It
does not wait for API or UI readiness before returning.

Confirm the stack:

```bash
docker compose ps
curl -i http://localhost:8000/health_check
```

Open:

- UI: <http://localhost:3000>
- Player list: <http://localhost:3000/players>
- Device logs: <http://localhost:3000/device-logs>
- Direct local API: <http://localhost:8000>

Seed repeatable example data if useful:

```bash
scripts/seed_db.sh
```

### Routine source changes

Compose runs built images, not bind-mounted source. Rebuild when Rust, web,
migration, dependency, or Docker files change:

```bash
docker compose build
scripts/start_local_instance.sh
```

For a configuration-only change supplied through `.env`, starting again is
enough for Compose to recreate affected containers.

### Logs

Follow structured API logs:

```bash
scripts/view_logs.sh
```

If the optional `bunyan` formatter is unavailable, the script shows raw logs.
Follow UI logs directly:

```bash
docker compose logs --no-log-prefix -f web
```

PostgreSQL and migration logs are also available through Compose:

```bash
docker compose logs postgres migrate
```

### Stop, resume, back up, and delete

Stop containers while preserving them and the PostgreSQL volume:

```bash
docker compose stop
```

Resume with:

```bash
scripts/start_local_instance.sh
```

Create manual local backups:

```bash
scripts/backup_logs.sh
scripts/backup_db.sh
```

They write private, gitignored files under `backups/`. See the
[script reference](scripts.md) for exact paths and limitations; there is no
restore helper.

Deleting the stack requires PostgreSQL to be running because the script first
backs it up with `docker compose exec`; resume the stack if it is stopped. If
either backup fails, deletion stops. Images, `.env`, and backup files remain.
To remove the containers, network, and database volume:

```bash
scripts/delete_local_instance.sh
```

The script requires an exact confirmation phrase.

Do not use `scripts/kill_db.sh` for this stack. That script targets the separate
standalone database used by native development.

## Shared DigitalOcean environment

[`spec.yaml`](../spec.yaml) is the desired App Platform configuration. The
[architecture guide](architecture.md#shared-digitalocean-environment) explains
its components and public routing.

The public hostname is assigned by DigitalOcean and is not stored in this
repository. If `doctl` is installed and authenticated, list the app ID and
origin with:

```bash
doctl apps list --format ID,Spec.Name,DefaultIngress
```

### Routine source deployment

Both services track the GitHub repository's `main` branch with
`deploy_on_push: true`. Merging or pushing a source commit to `main` asks App
Platform to build and deploy both components.

Before merging:

1. Run the relevant Rust and web checks from
   [Tests and checks](development.md#tests-and-checks).
2. Build both local images with `docker compose build` when Docker or runtime
   behavior changed.
3. If the schema changed, follow the manual migration procedure below.
4. If `spec.yaml` changed, validate and apply the stored app spec separately.

After deployment:

1. Confirm the deployment completed in App Platform.
2. Request `https://<app-origin>/api/health_check`.
3. Load at least one database-backed page such as `/players` or
   `/device-logs`.
4. Inspect both API and web runtime logs if either fails.

The extra data-page check matters: the API's health endpoint does not connect
to PostgreSQL, so it can be green while migrations or database credentials are
wrong.

### App spec changes

`deploy_on_push` deploys source but does not apply changes to the checked-in
`spec.yaml`. First validate the file and retrieve the live spec:

```bash
doctl apps spec validate spec.yaml
doctl apps spec get <app-id> --format yaml
```

Reconcile the live spec with the checked-in file first. Applying `spec.yaml`
can overwrite dashboard-only domains, alerts, or settings that are absent from
the repository. Review the proposed diff in DigitalOcean before applying
changes that resize, replace, or remove a component. Then update the existing
app:

```bash
doctl apps update <app-id> --spec spec.yaml --wait
```

See the
[App Spec reference](https://docs.digitalocean.com/products/app-platform/reference/app-spec/),
[`doctl apps spec validate`](https://docs.digitalocean.com/reference/doctl/reference/apps/spec/validate/),
and [`doctl apps update`](https://docs.digitalocean.com/reference/doctl/reference/apps/update/).

### Database migrations

The root Dockerfile contains a migration image stage for Compose, but the
DigitalOcean spec does not instantiate it as a deployment job. The API runtime
image also does not migrate on startup. Forward migrations are therefore a
manual, coordinated step.

Load the DigitalOcean database connection string into `DATABASE_URL` through a
secure shell or password manager. Do not commit it to `.env`, paste it into
chat, or leave it in shared logs.

Inspect status without changing the schema:

```bash
sqlx migrate info --no-dotenv
```

When the migration has been reviewed, tested on a fresh local database, and is
compatible with the currently deployed application, apply pending migrations:

```bash
sqlx migrate run --no-dotenv
unset DATABASE_URL
```

Then deploy the code that uses the new schema and verify a database-backed
request. Prefer expand-then-contract migrations that work with both the old and
new application during the deployment window. A destructive or incompatible
migration needs an explicit maintenance and rollback plan; this repository
cannot deploy schema and code as one atomic change.

`scripts/reset_remote_db.sh` is not a migration command. It permanently drops
all application data and SQLx's ledger from a disposable DigitalOcean database
before replaying every migration. Never run it for routine verification or
against a database whose data matters.

### Logs

Use the App Platform console, or retrieve a component's runtime logs with an
authenticated `doctl` session:

```bash
doctl apps logs <app-id> alleycat --type run
doctl apps logs <app-id> web --type run
```

See the official [`doctl apps logs` reference](https://docs.digitalocean.com/reference/doctl/reference/apps/logs/)
for deployment and build log options.

### Shared-environment safety

- The current HTTP routes have no authentication or authorization. Public API
  reads, device-log writes, and player registration are reachable through
  `/api`; registration also consumes finite PDN codes. Do not store sensitive
  data or treat a device MAC as verified identity.
- `main` deploys directly to the only shared internet environment; there is no
  separate staging topology.
- The repo has no remote database backup or restore helper. Establish the
  DigitalOcean backup/recovery policy before the data becomes important.
- Rolling application code back does not roll a database migration back.
- The only health endpoint is liveness-only; always verify a data path.

## Known operational gaps

- Add an App Platform pre-deploy migration job or another documented migration
  automation path.
- Add an authenticated readiness check that verifies database access without
  leaking details publicly.
- Add authentication and rate limiting before treating the public API as
  production-ready.
- Pin the native helper's PostgreSQL version and align CI's PostgreSQL 14 with
  PostgreSQL 18 in Compose and DigitalOcean.
- Add and test backup restoration, not only backup creation.
