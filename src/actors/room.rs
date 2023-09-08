use super::player::PlayerHandle;
use crate::{
    models::{ChipInt, Game, PlayerId, Table},
    server::messages::{PokerMessage, ServerMessage},
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::error;

pub type RoomId = String;

#[derive(Clone)]
pub struct RoomHandle {
    sender: mpsc::Sender<RoomMessage>,
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
        let msg = RoomMessage::GetTable { respond_to: send };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Room task has been killed")
    }

    pub async fn subscribe(&self) -> broadcast::Receiver<PokerMessage> {
        let (send, recv) = oneshot::channel();
        let msg = RoomMessage::Subscribe { respond_to: send };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Room task has been killed")
    }

    pub async fn sit_table(&mut self, id: PlayerId, chips: ChipInt) -> Table {
        let (send, recv) = oneshot::channel();
        let msg = RoomMessage::SitTable {
            id,
            chips,
            respond_to: send,
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Room task has been killed")
    }

    pub fn send_chat_message(&self, message: String, from: PlayerId) {
        let msg = RoomMessage::Chat { message, from };
        let _ = self.sender.try_send(msg);
    }
}

struct Room {
    receiver: mpsc::Receiver<RoomMessage>,
    players: Vec<PlayerHandle>,
    broadcast: broadcast::Sender<PokerMessage>,
    table: Table,
    game: Option<Game>,
}

enum RoomMessage {
    GetTable {
        respond_to: oneshot::Sender<Table>,
    },
    Subscribe {
        respond_to: oneshot::Sender<broadcast::Receiver<PokerMessage>>,
    },

    SitTable {
        id: PlayerId,
        chips: ChipInt,
        respond_to: oneshot::Sender<Table>,
    },
    Chat {
        from: PlayerId,
        message: String,
    },
}

impl Room {
    fn new(receiver: mpsc::Receiver<RoomMessage>, table: Table) -> Self {
        let (broadcast, _) = broadcast::channel(8);
        Room {
            receiver,
            table,
            broadcast,
            game: None,
            players: vec![],
        }
    }
    fn handle_message(&mut self, msg: RoomMessage) {
        match msg {
            RoomMessage::GetTable { respond_to } => {
                let _ = respond_to.send(self.table.clone());
            }

            RoomMessage::Subscribe { respond_to } => {
                let _ = respond_to.send(self.broadcast.subscribe());
            }

            RoomMessage::SitTable {
                id,
                chips,
                respond_to,
            } => {
                self.table.players.push((id, chips));
                let _ = respond_to.send(self.table.clone());
            }
            RoomMessage::Chat { from, message } => {
                let broadcast_msg = PokerMessage::ServerResponse(ServerMessage::Chat {
                    from: from.clone(),
                    message: message.clone(),
                });
                if let Err(e) = self.broadcast.send(broadcast_msg) {
                    error!(err = ?e, "Error broadcasting chat message");
                }
            }
        }
    }
}

async fn run(mut room: Room) {
    while let Some(msg) = room.receiver.recv().await {
        room.handle_message(msg);
    }
}
