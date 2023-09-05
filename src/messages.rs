use super::server::RoomId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PokerMessage {
    Lobby(LobbyMessage),
    Room(RoomWrapper),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LobbyMessage {
    Hello,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomWrapper {
    pub id: RoomId,
    pub payload: RoomMessage,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum RoomMessage {
    Chat(String),
    GameUpdate(GameEvent),
    PlayerAction(PlayerEvent),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameEvent {
    NewGame,
    DealCards((String, String)),
    CommunityCards {
        flop: Vec<String>,
        turn: String,
        river: String,
    },
    DeclareWinner {
        winner: String,
        cards: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlayerEvent {
    Bet(usize),
    Fold,
}
