//! Shared database lookups for the active event.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Read the current selection without preventing a later event switch.
/// No state row or a null selection both mean there is no active event.
pub async fn get_active_event_id(pool: &PgPool) -> Result<Option<Uuid>, sqlx::Error> {
    Ok(
        sqlx::query_scalar!("SELECT active_event_id FROM app_state WHERE id = 1")
            .fetch_optional(pool)
            .await?
            .flatten(),
    )
}

/// Read and hold the active-event selection until the transaction ends.
/// Concurrent readers are allowed; updates to the selection must wait.
/// This locks the selection in app_state, not the event row itself.
pub async fn lock_active_event(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Option<Uuid>, sqlx::Error> {
    Ok(
        sqlx::query_scalar!("SELECT active_event_id FROM app_state WHERE id = 1 FOR SHARE")
            .fetch_optional(&mut **transaction)
            .await?
            .flatten(),
    )
}
