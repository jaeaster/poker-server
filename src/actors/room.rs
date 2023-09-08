use crate::*;
use tokio::sync::{broadcast, mpsc, oneshot};

pub type RoomId = String;

#[derive(Clone)]
pub struct RoomHandle {
    sender: mpsc::Sender<RoomActorMessage>,
    pub id: RoomId,
}

impl RoomHandle {
    pub fn new(table: Table) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let room = Room::new(receiver, table.clone());
        tokio::spawn(run(room));

        Self {
            sender,
            id: table.id,
        }
    }

    pub async fn get_table(&self) -> Table {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::GetTable { respond_to: send };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Room task has been killed")
    }

    pub async fn subscribe(&self) -> broadcast::Receiver<PokerMessage> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::Subscribe { respond_to: send };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Room task has been killed")
    }

    pub async fn sit_table(&self, player: Player) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::SitTable {
            player,
            respond_to: send,
        };
        let _ = self.sender.try_send(msg);
        recv.await.expect("Room task has been killed")
    }

    pub fn send_chat_message(&self, message: String, from: PlayerId) -> Result<()> {
        let msg = RoomActorMessage::Chat { message, from };
        let _ = self.sender.try_send(msg);
        Ok(())
    }
}

struct Room {
    receiver: mpsc::Receiver<RoomActorMessage>,
    players: Vec<PlayerHandle>,
    broadcast: broadcast::Sender<PokerMessage>,
    table: Table,
    game: Option<Game>,
}

enum RoomActorMessage {
    GetTable {
        respond_to: oneshot::Sender<Table>,
    },
    Subscribe {
        respond_to: oneshot::Sender<broadcast::Receiver<PokerMessage>>,
    },

    SitTable {
        player: Player,
        respond_to: oneshot::Sender<Result<()>>,
    },
    Chat {
        from: PlayerId,
        message: String,
    },
}

impl Room {
    fn new(receiver: mpsc::Receiver<RoomActorMessage>, table: Table) -> Self {
        let (broadcast, _) = broadcast::channel(8);
        Room {
            receiver,
            table,
            broadcast,
            game: None,
            players: vec![],
        }
    }
    fn handle_message(&mut self, msg: RoomActorMessage) {
        match msg {
            RoomActorMessage::GetTable { respond_to } => {
                let _ = respond_to.send(self.table.clone());
            }

            RoomActorMessage::Subscribe { respond_to } => {
                let _ = respond_to.send(self.broadcast.subscribe());
            }

            RoomActorMessage::SitTable { player, respond_to } => {
                // TODO: Handle min and max buy-in
                // TODO: Handle "going south"
                if self.table.players.len() >= self.table.max_players {
                    debug!(player = ?player, "Max players at table");
                    let _ = respond_to.send(Err(eyre!("Table is full")));
                    return;
                }
                if self.table.players.iter().any(|p| p.id == player.id) {
                    debug!(player = ?player, "Player already sat");
                    let _ = respond_to.send(Err(eyre!("Already sitting at table")));
                    return;
                }

                let sit_table_msg =
                    PokerMessage::sit_table_broadcast(player.clone(), self.table.players.len());
                self.table.players.push(player);

                if let Err(e) = self.broadcast.send(sit_table_msg) {
                    error!(err = ?e, "Error broadcasting sat table");
                }

                // Start a new game if min_players have sat
                if self.table.players.len() == self.table.min_players {
                    if self.game.is_none() {
                        self.start_new_game()
                    } else {
                        panic!("Game should not be in progress!");
                    }
                }

                let _ = respond_to.send(Ok(()));
            }
            RoomActorMessage::Chat { from, message } => {
                let broadcast_msg = PokerMessage::chat_broadcast(from, message);
                if let Err(e) = self.broadcast.send(broadcast_msg) {
                    error!(err = ?e, "Error broadcasting chat message");
                }
            }
        }
    }

    fn start_new_game(&mut self) {
        let mut new_game = Game::new(
            self.table.id.clone(),
            self.table.players.clone(),
            0,
            self.table.small_blind,
            self.table.big_blind,
        );
        new_game.advance();

        let new_game_msg = PokerMessage::new_game(&self.table.id, &new_game);
        self.game = Some(new_game);

        if let Err(e) = self.broadcast.send(new_game_msg) {
            error!(err = ?e, "Error broadcasting new game");
        }
        // TODO: Send player's their cards
    }
}

async fn run(mut room: Room) {
    while let Some(msg) = room.receiver.recv().await {
        room.handle_message(msg);
    }
}
