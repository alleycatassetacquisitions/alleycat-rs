use crate::domain::player_email::PlayerEmail;
use crate::domain::player_name::PlayerName;
use uuid::Uuid;

pub struct Player {
    pub id: UUID,
    pub pdn_code: String,
    pub name: PlayerName,
    pub email: PlayerEmail,
    pub created_at: chrono::NaiveDateTime,
    pub mode: PlayerMode,
}

pub enum PlayerMode {
    Unassigned,
    Hunter,
    Bounty,
}
