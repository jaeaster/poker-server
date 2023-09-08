use crate::*;
use std::collections::HashMap;
use tokio::sync::mpsc;

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
        let _ = self.sender.try_send(PokerMessage::error(err.to_string()));
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
            PokerMessage::Lobby(msg) => self.handle_lobby_message(msg).await,
            PokerMessage::Room(RoomWrapper { room_id, payload }) => {
                self.handle_room_message(room_id, payload).await
            }
            msg @ PokerMessage::ServerResponse(_) => self.send_to_socket(msg),
        };

        if let Err(e) = result {
            debug!(err = ?e, "Invalid poker message");
            let _ = self.send_to_socket(PokerMessage::error(e.to_string()));
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
                Ok(())
            }
            RoomMessage::SitTable { chips } => {
                if chips > self.player.chips {
                    bail!("Insufficient Chips");
                }
                self.player.chips -= chips;
                let mut table_player = self.player.clone();
                table_player.chips = chips;
                room.sit_table(table_player.clone()).await
            }
            _ => unimplemented!(),
        }
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
                let tables_msg = PokerMessage::table_list(tables);
                let _ = self.send_to_socket(tables_msg);
            }
        }
        Ok(())
    }

    fn send_to_socket(&self, msg: PokerMessage) -> Result<()> {
        if let Err(e) = self.socket.try_send(msg) {
            error!(e = ?e, "Error sending table message to socket")
        }
        Ok(())
    }
}

async fn run(mut player: PlayerActor) {
    while let Some(msg) = player.receiver.recv().await {
        let _ = player.handle_message(msg).await;
    }
}
