use crate::*;

pub type PlayerId = String;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Player {
    pub id: PlayerId,
    pub username: String,
}

impl Player {
    pub fn new(id: String, username: String) -> Self {
        Self { id, username }
    }
}

impl From<GamePlayer> for Player {
    fn from(game_player: GamePlayer) -> Self {
        game_player.info
    }
}
