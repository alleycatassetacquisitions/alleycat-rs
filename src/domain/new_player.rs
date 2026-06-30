use crate::domain::player_email::PlayerEmail;
use crate::domain::player_name::PlayerName;

pub struct NewPlayer {
    pub email: PlayerEmail,
    pub name: PlayerName,
}
