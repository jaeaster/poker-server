use crate::*;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct PlayerHandle {
    pub sender: mpsc::Sender<PokerMessage>,
    pub id: PlayerId,
}

impl PlayerHandle {
    pub fn new(
        player: Player,
        rooms: RegistryHandle<RoomId, RoomHandle>,
        socket: mpsc::Sender<PokerMessage>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(*CHANNEL_SIZE);
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
        // TODO: Fix error_lobby, error_room
        let _ = self
            .sender
            .try_send(PokerMessage::error_lobby(err.to_string()));
        Ok(())
    }
}

struct PlayerActor {
    room_registry: RegistryHandle<RoomId, RoomHandle>,
    player: Player,
    receiver: mpsc::Receiver<PokerMessage>,
    socket: mpsc::Sender<PokerMessage>,
}

impl PlayerActor {
    fn new(
        player: Player,
        rooms: RegistryHandle<RoomId, RoomHandle>,
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
        match poker_msg {
            PokerMessage::Client(msg) => match msg {
                Either::Lobby(lobby_msg) => {
                    if let Err(e) = self.handle_lobby_message(lobby_msg).await {
                        self.send_to_socket(PokerMessage::error_lobby(e.to_string()));
                    }
                }
                Either::Room(room_msg) => {
                    let room_id = room_msg.room_id.clone();
                    if let Err(e) = self.handle_room_message(room_msg).await {
                        self.send_to_socket(PokerMessage::error_room(room_id, e.to_string()))
                    }
                }
            },
            msg @ PokerMessage::Server(_) => self.send_to_socket(msg),
        };
    }

    async fn handle_lobby_message(&self, msg: ClientLobby) -> Result<()> {
        match msg {
            ClientLobby::GetTables => {
                let rooms = self.room_registry.get_all().await;
                let mut tables = vec![];
                for room in rooms {
                    let table = room.get_table().await;
                    tables.push(table);
                }
                let tables_msg = PokerMessage::table_list(tables);
                self.send_to_socket(tables_msg);
            }
        }
        Ok(())
    }

    async fn handle_room_message(
        &mut self,
        RoomMessage { room_id, payload }: RoomMessage<ClientRoomPayload>,
    ) -> Result<()> {
        debug!(room = room_id, "Handling message for room: {:?}", payload);

        let room = self
            .room_registry
            .get(room_id.clone())
            .await
            .ok_or(eyre!("Not a valid room id"))?;

        match payload {
            ClientRoomPayload::Chat(message) => {
                room.send_chat_message(message, self.player.id.clone())
                    .await
            }
            ClientRoomPayload::Subscribe => {
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
            ClientRoomPayload::SitTable { chips } => {
                // TODO: Read chips from smart contract
                if chips > *DEFAULT_CHIPS {
                    bail!("Insufficient Chips");
                }
                let table_player = self.player.clone();
                room.sit_table(table_player).await
            }
            ClientRoomPayload::Bet(chips) => room.bet(self.player.clone(), chips).await,
            ClientRoomPayload::Fold => room.fold(self.player.clone()).await,

            ClientRoomPayload::SitOutNextHand(value) => {
                room.sit_out_next_hand(self.player.clone(), value).await
            }
            ClientRoomPayload::SitOutNextBigBlind(value) => {
                room.sit_out_next_big_blind(self.player.clone(), value)
                    .await
            }
            ClientRoomPayload::WaitForBigBlind(value) => {
                room.wait_for_big_blind(self.player.clone(), value).await
            }
            ClientRoomPayload::CheckFold(value) => {
                room.check_fold(self.player.clone(), value).await
            }
            ClientRoomPayload::CallAny(value) => room.call_any(self.player.clone(), value).await,
        }
    }

    fn send_to_socket(&self, msg: PokerMessage) {
        if let Err(e) = self.socket.try_send(msg) {
            error!(e = ?e, "Error sending table message to socket")
        }
    }
}

async fn run(mut player: PlayerActor) {
    while let Some(msg) = player.receiver.recv().await {
        let _ = player.handle_message(msg).await;
    }
}
