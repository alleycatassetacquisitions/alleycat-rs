mod get;
mod register;
mod update;

pub use get::*;
pub use register::*;
pub use update::*;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
pub(super) struct PlayerResponse {
    pub(super) id: Uuid,
    pub(super) event_id: Uuid,
    /// Four-digit PIN, including leading zeros. Fixed for this registration.
    pub(super) pdn_code: String,
    pub(super) name: String,
    pub(super) email: Option<String>,
    pub(super) created_at: DateTime<Utc>,
    pub(super) team_id: Option<Uuid>,
    /// Current team name; null when unassigned.
    pub(super) team_name: Option<String>,
    /// Registration/device status. Changing it never rewrites match history.
    pub(super) active: bool,
}
