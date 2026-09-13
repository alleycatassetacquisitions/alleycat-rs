//! Replace scripts/init_db.sh: initialize the standalone development database.
//! Preserve POSTGRES_PORT, SUPERUSER, SUPERUSER_PWD, APP_USER, APP_USER_PWD,
//! and APP_DB_NAME overrides. Avoid printing passwords or connection strings.
//! Unless SKIP_DOCKER is set, check container/port conflicts, start Postgres,
//! wait with a timeout, and create the application role with CREATEDB.
//! With SKIP_DOCKER, reuse the existing server/role without Docker or TTY access
//! so CI can initialize its service database. Create the DB and apply migrations.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("native db init is not implemented yet")
}
