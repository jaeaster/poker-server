use crate::*;
use tokio::sync::{broadcast, mpsc, oneshot};

pub type RoomId = String;

#[derive(Clone)]
pub struct RoomHandle {
    sender: mpsc::Sender<RoomActorMessage>,
    pub id: RoomId,
}

impl RoomHandle {
    pub fn new(table: Table, player_registry: PlayerRegistryHandle) -> Self {
        let (sender, receiver) = mpsc::channel(*CHANNEL_SIZE);
        let room = Room::new(receiver, table.clone(), player_registry);
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

    pub async fn send_chat_message(&self, message: String, from: PlayerId) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::Chat {
            message,
            from,
            respond_to: send,
        };
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

    pub async fn bet(&self, player: Player, chips: ChipInt) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::Bet {
            player,
            chips,
            respond_to: send,
        };
        let _ = self.sender.try_send(msg);
        recv.await.expect("Room task has been killed")
    }

    pub async fn fold(&self, player: Player) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::Fold {
            player,
            respond_to: send,
        };
        let _ = self.sender.try_send(msg);
        recv.await.expect("Room task has been killed")
    }
}

struct Room {
    receiver: mpsc::Receiver<RoomActorMessage>,
    player_registry: PlayerRegistryHandle,
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
        respond_to: oneshot::Sender<Result<()>>,
    },
    Bet {
        player: Player,
        chips: ChipInt,
        respond_to: oneshot::Sender<Result<()>>,
    },
    Fold {
        player: Player,
        respond_to: oneshot::Sender<Result<()>>,
    },
}

impl Room {
    fn new(
        receiver: mpsc::Receiver<RoomActorMessage>,
        table: Table,
        player_registry: PlayerRegistryHandle,
    ) -> Self {
        let (broadcast, _) = broadcast::channel(*CHANNEL_SIZE);
        Room {
            receiver,
            table,
            broadcast,
            player_registry,
            game: None,
        }
    }
    async fn handle_message(&mut self, msg: RoomActorMessage) {
        match msg {
            RoomActorMessage::GetTable { respond_to } => {
                let _ = respond_to.send(self.table.clone());
            }
            RoomActorMessage::Subscribe { respond_to } => {
                let _ = respond_to.send(self.broadcast.subscribe());
            }
            RoomActorMessage::Chat {
                from,
                message,
                respond_to,
            } => {
                let _ = respond_to.send(self.handle_chat(from, message));
            }
            RoomActorMessage::SitTable { player, respond_to } => {
                let _ = respond_to.send(self.handle_sit(player).await);
            }
            RoomActorMessage::Bet {
                player,
                chips,
                respond_to,
            } => {
                let _ = respond_to.send(self.handle_bet(player, chips).await);
            }
            RoomActorMessage::Fold { player, respond_to } => {
                let _ = respond_to.send(self.handle_fold(player).await);
            }
        }
    }

    fn handle_chat(&mut self, from: String, message: String) -> Result<()> {
        let broadcast_msg = PokerMessage::chat_broadcast(self.table.id.clone(), from, message);
        if let Err(e) = self.broadcast.send(broadcast_msg) {
            error!(err = ?e, "Error broadcasting chat message");
        }
        Ok(())
    }

    async fn handle_sit(&mut self, player: Player) -> Result<()> {
        // TODO: Handle min and max buy-in
        // TODO: Handle chips from smart contract
        // TODO: Handle "going south"
        if self.table.players.len() >= self.table.max_players {
            debug!(player = ?player, "Max players at table");
            bail!("Table is full")
        }
        if self.table.players.iter().any(|p| p.id == player.id) {
            debug!(player = ?player, "Player already sat");
            bail!("Already sitting at table")
        }

        let sit_table_msg = PokerMessage::sit_table_broadcast(
            self.table.id.clone(),
            player.clone(),
            self.table.players.len(),
        );

        self.table.players.push(player);

        if let Err(e) = self.broadcast.send(sit_table_msg) {
            error!(err = ?e, "Error broadcasting sat table");
        }

        // Start a new game if min_players have sat
        if self.table.players.len() == self.table.min_players {
            if self.game.is_none() {
                self.start_new_game().await
            } else {
                panic!("Game should not be in progress!");
            }
        }
        Ok(())
    }

    async fn start_new_game(&mut self) {
        let new_dealer_idx = self.game.as_ref().map_or(0, |game| {
            game.state.dealer_idx + 1 % self.table.players.len()
        });

        let mut new_game = Game::new(
            self.table.id.clone(),
            self.table.players.clone(),
            new_dealer_idx,
            self.table.small_blind,
            self.table.big_blind,
        );
        // Advance to preflop and take blinds
        new_game.advance_round();

        let new_game_msg = PokerMessage::new_game(self.table.id.clone(), &new_game);

        if let Err(e) = self.broadcast.send(new_game_msg) {
            error!(err = ?e, "Error broadcasting new game");
        }
        for (player, hand) in self.table.players.iter().zip(new_game.state.hands.clone()) {
            let deal_hand_msg = PokerMessage::deal_hand(self.table.id.clone(), hand);

            if let Err(e) = self.send_to_player(&player.id, deal_hand_msg).await {
                error!(err = ?e, "Error sending deal hand");
            }
        }

        self.game = Some(new_game);
    }

    fn valid_game_and_turn(&mut self, player: &Player) -> Result<&mut Game> {
        if let Some(game) = &mut self.game {
            let round = game.state.current_round_data();
            if self.table.players.get(round.to_act_idx).unwrap().id != player.id {
                bail!("Not your turn")
            }
            Ok(game)
        } else {
            bail!("Game is not active")
        }
    }

    async fn handle_bet(&mut self, player: Player, chips: ChipInt) -> Result<()> {
        let room_id = self.table.id.clone();
        let game = self.valid_game_and_turn(&player)?;
        match game.bet(chips) {
            Ok(additional_bet) => {
                let game_update_msg = PokerMessage::game_update(room_id, game);
                let _ = self.broadcast.send(game_update_msg);
                if self.game.as_ref().unwrap().is_over() {
                    self.start_new_game().await;
                }
                Ok(())
            }
            Err(e) => {
                bail!(e.to_string())
            }
        }
    }

    async fn handle_fold(&mut self, player: Player) -> Result<()> {
        let room_id = self.table.id.clone();
        let game = self.valid_game_and_turn(&player)?;
        game.fold();
        let game_update_msg = PokerMessage::game_update(room_id, game);
        let _ = self.broadcast.send(game_update_msg);
        if self.game.as_ref().unwrap().is_over() {
            self.start_new_game().await;
        }
        Ok(())
    }

    async fn send_to_player(&self, id: &PlayerId, msg: PokerMessage) -> Result<()> {
        self.player_registry
            .get(id.clone())
            .await
            .ok_or(eyre!("Player connection closed"))
            .map(|p| p.send_message(msg))?
    }
}

async fn run(mut room: Room) {
    while let Some(msg) = room.receiver.recv().await {
        room.handle_message(msg).await;
    }
}
