mod get;
mod register;
mod update;

pub use get::*;
pub use register::*;
pub use update::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow, utoipa::ToSchema)]
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
    /// Registration status. Changing it never rewrites match history.
    pub(super) active: bool,
    /// Current assignments, ordered by category name then category ID. Empty when unassigned.
    /// Includes only assigned categories; names reflect current category and affiliation names.
    #[sqlx(json)]
    affiliations: Vec<PlayerAffiliation>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub(super) struct PlayerAffiliation {
    category_id: Uuid,
    category_name: String,
    affiliation_id: Uuid,
    affiliation_name: String,
}
