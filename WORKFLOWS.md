# App workflows

Run commands from the repository root with `cargo xtask`.
Keep the existing scripts until their replacements are implemented and verified.

**Ready** = implemented. **Partial** = some behavior remains.
**TBD** = a placeholder that reports “not implemented yet” and performs no work.

## Commands and source scripts

Click a command to edit its Rust file; click its script to see the behavior to port.

| Command | Status | Existing script |
| --- | --- | --- |
| [stack doctor](xtask/src/stack/doctor.rs) | Ready | Existing prerequisite checks; used by `stack start`. |
| [stack start](xtask/src/stack/start.rs) | Ready | [start_local_instance.sh (before deletion)](https://github.com/alleycatassetacquisitions/alleycat-rs/blob/e4cea9d54dc8a43dfb794431884472c58fbe6368/scripts/start_local_instance.sh) |
| [stack db seed](xtask/src/stack/db/seed.rs) | Partial: Compose seeding works; friendly preflight checks remain. | [seed_db.sh](scripts/seed_db.sh), Compose branch |
| [stack db backup](xtask/src/stack/db/backup.rs) | TBD | [backup_db.sh](scripts/backup_db.sh) |
| [stack db reset](xtask/src/stack/db/reset.rs) | Ready | Confirms deletion, tears down the local stack and volumes, then builds, starts, migrates, and seeds. No backup. |
| [stack logs](xtask/src/stack/logs/follow.rs) | Ready | Streams raw app logs without Compose colors or prefixes; pipe to a formatter if desired. |
| [stack logs backup](xtask/src/stack/logs/backup.rs) | TBD | [backup_logs.sh](scripts/backup_logs.sh) |
| [stack delete](xtask/src/stack/delete.rs) | Partial: confirmation only; then reports not implemented. | [delete_local_instance.sh](scripts/delete_local_instance.sh) |
| [native db init](xtask/src/native/db/init.rs) | TBD | [init_db.sh](scripts/init_db.sh), including migrations |
| [native db seed](xtask/src/native/db/seed.rs) | TBD | [seed_db.sh](scripts/seed_db.sh), URL branch |
| [native db reset](xtask/src/native/db/reset.rs) | TBD | [reset_db.sh](scripts/reset_db.sh) |
| [native db delete](xtask/src/native/db/delete.rs) | TBD | [kill_db.sh](scripts/kill_db.sh) |
| [native db clean-tests](xtask/src/native/db/clean_tests.rs) | TBD | [clean_test_dbs.sh](scripts/clean_test_dbs.sh) |
| [cloud db seed](xtask/src/cloud/db/seed.rs) | TBD | [seed_db.sh](scripts/seed_db.sh), URL branch |
| [cloud db reset](xtask/src/cloud/db/reset.rs) | TBD | [reset_remote_db.sh](scripts/reset_remote_db.sh), including migrations |
| [sqlx prepare](xtask/src/sqlx/prepare.rs) | TBD | [prepare_sqlx.sh](scripts/prepare_sqlx.sh) |

Menu files: [root](xtask/src/main.rs), [stack](xtask/src/stack/mod.rs),
[stack db](xtask/src/stack/db/mod.rs), [logs](xtask/src/stack/logs/mod.rs),
[native](xtask/src/native/mod.rs), [native db](xtask/src/native/db/mod.rs),
[cloud](xtask/src/cloud/mod.rs), [cloud db](xtask/src/cloud/db/mod.rs),
[sqlx](xtask/src/sqlx/mod.rs).

Shared code: [process helpers](xtask/src/command.rs), [environment](xtask/src/env.rs),
[project paths](xtask/src/project.rs), [seed runner](xtask/src/seed.rs),
[seed SQL](xtask/seed_db.sql).

## Porting notes

The existing seed script reads [xtask/seed_db.sql](xtask/seed_db.sql). Startup
instructions use `cargo xtask stack start`; follow logs with `cargo xtask stack logs`.

Keep migrations within startup, native initialization, and cloud reset for now.
`stack start` builds images before starting the containers. Stop remains
`docker compose stop`.
Startup runs `sqlx migrate info` after starting PostgreSQL and before running
migrations or starting the app. A mix of `/installed` and `/pending` statuses
requires `cargo xtask stack db reset`. Only pending migrations are treated as a
fresh database and initialize normally, including after reset. Only installed
migrations allow startup with existing data.
When porting SQLx preparation, note that its script resets the native DB first;
that destructive step should remain explicit in the task's behavior.
Follow [AGENTS.md](AGENTS.md) for the remote reset inventory. Never run remote
reset as routine verification.
