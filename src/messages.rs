use crate::*;

mod client;
mod server;

pub use client::*;
pub use server::*;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum Either<C, S> {
    Lobby(C),
    Room(S),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum PokerMessage {
    Client(Either<ClientLobby, RoomMessage<ClientRoomPayload>>),
    Server(Either<ServerLobby, RoomMessage<ServerRoomPayload>>),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoomMessage<RoomPayload> {
    pub room_id: RoomId,

    #[serde(flatten)]
    pub payload: RoomPayload,
}
