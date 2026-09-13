//! Apply pending migrations to the Compose database without resetting it.
//! Check the Compose configuration and database readiness from project_root().
//! Use the migration service and ensure its image includes current migrations;
//! migrations are copied into the image, not mounted from the checkout.
//! Wait for migration completion and propagate errors. Keep application startup
//! separate so this operation can be reused without restarting the API.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("stack db migrate is not implemented yet")
}
