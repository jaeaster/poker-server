use crate::*;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ClientLobby {
    GetTables,
}
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ClientRoomPayload {
    Subscribe,
    Chat(String),
    SitTable { chips: ChipInt },
    PlayerAction(PlayerEvent),
    SitOutNextHand(bool),
    SitOutNextBigBlind(bool),
    WaitForBigBlind(bool),
    CheckFold(bool),
    CallAny(bool),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum PlayerEvent {
    Bet(ChipInt),
    Fold,
}

impl PokerMessage {
    pub fn get_tables() -> Self {
        Self::Client(Either::Lobby(ClientLobby::GetTables))
    }

    pub fn subscribe_room(room_id: RoomId) -> Self {
        Self::Client(Either::Room(RoomMessage {
            room_id,
            payload: ClientRoomPayload::Subscribe,
        }))
    }

    pub fn chat(room_id: RoomId, message: String) -> Self {
        Self::Client(Either::Room(RoomMessage {
            room_id,
            payload: ClientRoomPayload::Chat(message),
        }))
    }

    pub fn sit_table(room_id: RoomId, chips: ChipInt) -> Self {
        Self::Client(Either::Room(RoomMessage {
            room_id,
            payload: ClientRoomPayload::SitTable { chips },
        }))
    }

    pub fn bet(room_id: RoomId, bet: ChipInt) -> Self {
        Self::Client(Either::Room(RoomMessage {
            room_id,
            payload: ClientRoomPayload::PlayerAction(PlayerEvent::Bet(bet)),
        }))
    }

    pub fn fold(room_id: RoomId) -> Self {
        Self::Client(Either::Room(RoomMessage {
            room_id,
            payload: ClientRoomPayload::PlayerAction(PlayerEvent::Fold),
        }))
    }
}
