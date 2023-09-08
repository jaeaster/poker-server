use super::{RoomHandle, RoomId};
use crate::models::{Player, PlayerId};
use crate::server::messages::*;
use eyre::{eyre, Result};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error};

#[derive(Clone)]
pub struct PlayerHandle {
    sender: mpsc::Sender<PokerMessage>,
    id: PlayerId,
}

impl PlayerHandle {
    pub fn new(
        player: Player,
        rooms: HashMap<RoomId, RoomHandle>,
        socket: mpsc::Sender<PokerMessage>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let player_actor = PlayerActor::new(player.clone(), rooms, receiver, socket);
        tokio::spawn(run(player_actor));

        Self {
            sender,
            id: player.id,
        }
    }

    pub fn send_message(&self, msg: PokerMessage) -> Result<()> {
        self.sender.try_send(msg)?;
        Ok(())
    }

    pub fn send_error(&self, err: eyre::Error) -> Result<()> {
        debug!(err = ?err, "Responding with error to player");
        let _ = self
            .sender
            .try_send(PokerMessage::ServerResponse(ServerMessage::Error(
                err.to_string(),
            )));
        Ok(())
    }
}

struct PlayerActor {
    room_registry: HashMap<RoomId, RoomHandle>,
    player: Player,
    receiver: mpsc::Receiver<PokerMessage>,
    socket: mpsc::Sender<PokerMessage>,
}

impl PlayerActor {
    fn new(
        player: Player,
        rooms: HashMap<RoomId, RoomHandle>,
        receiver: mpsc::Receiver<PokerMessage>,
        socket: mpsc::Sender<PokerMessage>,
    ) -> Self {
        PlayerActor {
            player,
            room_registry: rooms,
            receiver,
            socket,
        }
    }

    async fn handle_message(&mut self, poker_msg: PokerMessage) {
        let result: Result<()> = match poker_msg {
            PokerMessage::Lobby(msg) => {
                if let Err(e) = self.handle_lobby_message(msg).await {
                    error!(err = ?e, "Error handling lobby message");
                }
                Ok(())
            }
            PokerMessage::Room(RoomWrapper { room_id, payload }) => {
                if let Err(e) = self.handle_room_message(room_id, payload).await {
                    error!(err = ?e, "Error handling room message");
                }
                Ok(())
            }
            msg @ PokerMessage::ServerResponse(_) => {
                if let Err(e) = self.socket.try_send(msg) {
                    error!(err = ?e, "Error sending to socket");
                }
                Ok(())
            }
        };

        if let Err(e) = result {
            error!(err = ?e, "Error handling poker message");
            let _ = self
                .socket
                .try_send(PokerMessage::ServerResponse(ServerMessage::Error(
                    e.to_string(),
                )));
        }
    }

    async fn handle_room_message(&mut self, room_id: RoomId, msg: RoomMessage) -> Result<()> {
        debug!(room = room_id, "Handling message for room: {:?}", msg);

        let room = self
            .room_registry
            .get(&room_id)
            .ok_or(eyre!("Not a valid room id"))?;

        match msg {
            RoomMessage::Chat(message) => room.send_chat_message(message, self.player.id.clone()),
            RoomMessage::Subscribe => {
                let mut subscription = room.subscribe().await;
                let socket = self.socket.clone();
                debug!(room = room.id, "Subscribing to room");
                tokio::spawn(async move {
                    while let Ok(msg) = subscription.recv().await {
                        debug!("Broadcasting message to player's socket");
                        if let Err(e) = socket.send(msg).await {
                            error!(err = ?e, "Error broadcasting to socket");
                            break;
                        }
                    }
                });
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    async fn handle_lobby_message(&self, msg: LobbyMessage) -> Result<()> {
        match msg {
            LobbyMessage::GetTables => {
                let rooms = self.room_registry.values();
                let mut tables = vec![];
                for room in rooms {
                    let table = room.get_table().await;
                    tables.push(table);
                }
                let tables_msg = PokerMessage::ServerResponse(ServerMessage::TableList(tables));
                if let Err(e) = self.socket.try_send(tables_msg) {
                    error!(e = ?e, "Error sending table message to socket")
                }
            }
        }
        Ok(())
    }
}

async fn run(mut player: PlayerActor) {
    while let Some(msg) = player.receiver.recv().await {
        let _ = player.handle_message(msg).await;
    }
}
