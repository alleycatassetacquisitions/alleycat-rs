use crate::seed::seed_via_compose;
use crate::stack::db::seed;
use anyhow::Result;

pub fn run() -> Result<()> {
    seed::seed_via_compose()
}
