use crate::domain::player_email::PlayerEmail;
use crate::domain::player_name::PlayerName;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct Player {
    pub id: Uuid,
    pub pdn_code: String,
    pub name: PlayerName,
    pub email: Option<PlayerEmail>,
    pub created_at: DateTime<Utc>,
    pub team: PlayerTeam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, serde::Serialize, utoipa::ToSchema)]
#[sqlx(type_name = "player_team", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PlayerTeam {
    Unassigned,
    Hunter,
    Bounty,
}
