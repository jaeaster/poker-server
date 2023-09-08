use crate::models::Table;
use crate::{PlayerId, RoomId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum PokerMessage {
    Lobby(LobbyMessage),
    Room(RoomWrapper),
    ServerResponse(ServerMessage),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    TableList(Vec<Table>),
    Chat { from: PlayerId, message: String },
    GameUpdate(GameEvent),
    PlayerAction(PlayerEvent),
    Error(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum LobbyMessage {
    GetTables,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct RoomWrapper {
    pub room_id: RoomId,

    #[serde(flatten)]
    pub payload: RoomMessage,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum RoomMessage {
    Subscribe,
    Chat(String),
    GameUpdate(GameEvent),
    PlayerAction(PlayerEvent),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum GameEvent {
    NewGame,
    DealCards(String, String),
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

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum PlayerEvent {
    Bet(usize),
    Fold,
}
