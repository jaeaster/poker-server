use crate::*;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{sleep, Duration};

pub type RoomId = String;

enum RoomActorMessage {
    GetTable {
        respond_to: oneshot::Sender<TableConfig>,
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
    SitOutNextHand {
        player: Player,
        value: bool,
        respond_to: oneshot::Sender<Result<()>>,
    },

    SitOutNextBigBlind {
        player: Player,
        value: bool,
        respond_to: oneshot::Sender<Result<()>>,
    },

    WaitForBigBlind {
        player: Player,
        value: bool,
        respond_to: oneshot::Sender<Result<()>>,
    },

    CheckFold {
        player: Player,
        value: bool,
        respond_to: oneshot::Sender<Result<()>>,
    },

    CallAny {
        player: Player,
        value: bool,
        respond_to: oneshot::Sender<Result<()>>,
    },
}

#[derive(Clone)]
pub struct RoomHandle {
    sender: mpsc::Sender<RoomActorMessage>,
    pub id: RoomId,
}

impl RoomHandle {
    pub fn new(
        table: Table,
        player_registry: RegistryHandle<PlayerId, PlayerHandle>,
        room_registry: RegistryHandle<RoomId, RoomHandle>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(*CHANNEL_SIZE);
        let id = table.id().clone();
        let room = Room::new(receiver, table, player_registry, room_registry);
        tokio::spawn(run(room));

        Self { sender, id }
    }

    pub async fn get_table(&self) -> TableConfig {
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

    pub async fn sit_out_next_hand(&self, player: Player, value: bool) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::SitOutNextHand {
            player,
            value,
            respond_to: send,
        };
        let _ = self.sender.try_send(msg);
        recv.await.expect("Room task has been killed")
    }
    pub async fn sit_out_next_big_blind(&self, player: Player, value: bool) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::SitOutNextBigBlind {
            player,
            value,
            respond_to: send,
        };
        let _ = self.sender.try_send(msg);
        recv.await.expect("Room task has been killed")
    }
    pub async fn wait_for_big_blind(&self, player: Player, value: bool) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::WaitForBigBlind {
            player,
            value,
            respond_to: send,
        };
        let _ = self.sender.try_send(msg);
        recv.await.expect("Room task has been killed")
    }
    pub async fn check_fold(&self, player: Player, value: bool) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::CheckFold {
            player,
            value,
            respond_to: send,
        };
        let _ = self.sender.try_send(msg);
        recv.await.expect("Room task has been killed")
    }
    pub async fn call_any(&self, player: Player, value: bool) -> Result<()> {
        let (send, recv) = oneshot::channel();
        let msg = RoomActorMessage::CallAny {
            player,
            value,
            respond_to: send,
        };
        let _ = self.sender.try_send(msg);
        recv.await.expect("Room task has been killed")
    }
}

struct Room {
    receiver: mpsc::Receiver<RoomActorMessage>,
    player_registry: RegistryHandle<PlayerId, PlayerHandle>,
    room_registry: RegistryHandle<RoomId, RoomHandle>,
    broadcast: broadcast::Sender<PokerMessage>,
    table: Table,
    turn_timer_cancel: Option<mpsc::Sender<()>>,
}

impl Room {
    fn new(
        receiver: mpsc::Receiver<RoomActorMessage>,
        table: Table,
        player_registry: RegistryHandle<PlayerId, PlayerHandle>,
        room_registry: RegistryHandle<RoomId, RoomHandle>,
    ) -> Self {
        let (broadcast, _) = broadcast::channel(*CHANNEL_SIZE);
        Room {
            receiver,
            table,
            broadcast,
            player_registry,
            room_registry,
            turn_timer_cancel: None,
        }
    }

    fn id(&self) -> &RoomId {
        self.table.id()
    }

    async fn handle_message(&mut self, msg: RoomActorMessage) {
        match msg {
            RoomActorMessage::GetTable { respond_to } => {
                let _ = respond_to.send(self.table.config.clone());
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
            RoomActorMessage::SitOutNextHand {
                player,
                value,
                respond_to,
            } => {
                let _ = respond_to.send(self.table.set_sit_out_next_hand(&player, value));
            }

            RoomActorMessage::SitOutNextBigBlind {
                player,
                value,
                respond_to,
            } => {
                let _ = respond_to.send(self.table.set_sit_out_next_big_blind(&player, value));
            }

            RoomActorMessage::WaitForBigBlind {
                player,
                value,
                respond_to,
            } => {
                let _ = respond_to.send(self.table.set_wait_for_big_blind(&player, value));
            }

            RoomActorMessage::CheckFold {
                player,
                value,
                respond_to,
            } => {
                let _ = respond_to.send(self.table.set_check_fold(&player, value));
            }
            RoomActorMessage::CallAny {
                player,
                value,
                respond_to,
            } => {
                let _ = respond_to.send(self.table.set_call_any(&player, value));
            }
        }
    }

    fn handle_chat(&mut self, from: String, message: String) -> Result<()> {
        let broadcast_msg = PokerMessage::chat_broadcast(self.id().clone(), from, message);
        if let Err(e) = self.broadcast.send(broadcast_msg) {
            error!(err = ?e, "Error broadcasting chat message");
        }
        Ok(())
    }

    async fn handle_sit(&mut self, player: Player) -> Result<()> {
        // TODO: Handle min and max buy-in
        // TODO: Handle chips from smart contract
        // TODO: Handle "going south"
        if self.table.num_players() >= self.table.max_players() {
            debug!(player = ?player, "Max players at table");
            bail!("Table is full")
        }
        if self.table.players.iter().any(|p| p.info.id == player.id) {
            debug!(player = ?player, "Player already sat");
            bail!("Already sitting at table")
        }

        let sit_table_msg = PokerMessage::sit_table_broadcast(
            self.table.id().clone(),
            player.clone(),
            self.table.players.len(),
        );

        self.table.players.push(player.into());

        if let Err(e) = self.broadcast.send(sit_table_msg) {
            error!(err = ?e, "Error broadcasting sat table");
        }

        // Try starting a new game, which will fail in most cases
        let _ = self.try_start_new_game().await;
        Ok(())
    }

    async fn try_start_new_game(&mut self) -> Result<()> {
        if self.table.game().is_some() && !self.table.game().unwrap().is_over() {
            bail!("Game is already in progress");
        }

        self.table.start_new_game()?;

        self.run_turn_timer(self.table.current_player().unwrap().clone())
            .await;
        let new_game_msg = PokerMessage::new_game(self.id().clone(), self.table.game().unwrap());

        if let Err(e) = self.broadcast.send(new_game_msg) {
            error!(err = ?e, "Error broadcasting new game");
        }
        for (player, hand) in self.table.game().unwrap().players_hands() {
            let deal_hand_msg = PokerMessage::deal_hand(self.id().clone(), hand.clone());

            if let Err(e) = self.send_to_player(&player.id, deal_hand_msg).await {
                error!(err = ?e, "Error sending deal hand");
            }
        }
        Ok(())
    }

    async fn handle_bet(&mut self, player: Player, chips: ChipInt) -> Result<()> {
        let room_id = self.id().clone();
        if let Some(game) = self.table.game_mut() {
            if game.is_players_turn(&player) {
                match game.bet(chips) {
                    Ok(additional_bet) => {
                        let game_update_msg = PokerMessage::game_update(room_id, game);
                        let _ = self.broadcast.send(game_update_msg);
                        self.run_turn_timer(player).await;
                        Ok(())
                    }
                    Err(e) => {
                        bail!(e.to_string())
                    }
                }
            } else {
                bail!("Not your turn")
            }
        } else {
            bail!("Game is not active")
        }
    }

    async fn handle_fold(&mut self, player: Player) -> Result<()> {
        let room_id = self.id().clone();
        if let Some(game) = self.table.game_mut() {
            if game.is_players_turn(&player) {
                game.fold();
                let game_update_msg = PokerMessage::game_update(room_id, game);
                let _ = self.broadcast.send(game_update_msg);

                if game.is_over() {
                    // Try starting a new game
                    // This fails if not enough players for the next game
                    let _ = self.try_start_new_game().await;
                } else {
                    self.run_turn_timer(player).await;
                }

                Ok(())
            } else {
                bail!("Not your turn")
            }
        } else {
            bail!("Game is not active")
        }
    }

    async fn send_to_player(&self, id: &PlayerId, msg: PokerMessage) -> Result<()> {
        self.player_registry
            .get(id.clone())
            .await
            .ok_or(eyre!("Player connection closed"))
            .map(|p| p.send_message(msg))?
    }

    async fn run_turn_timer(&mut self, player: Player) {
        // Cancel previous timer if exists
        if let Some(cancel) = self.turn_timer_cancel.clone() {
            let _ = cancel.try_send(());
        }
        let duration = Duration::from_secs(*TURN_TIMEOUT); // 30 seconds
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
        self.turn_timer_cancel = Some(cancel_tx);

        // Get self-referential handle for the timer callback
        let self_handle = self
            .room_registry
            .get(self.id().clone())
            .await
            .expect("Room should be registered");

        // Run timer
        tokio::spawn(async move {
            debug!("Timer running!");
            tokio::select! {
                _ = sleep(duration) => {
                    // Time's up. Send 'fold' message to Room actor.
                    let _ = self_handle.fold(player).await;
                },
                _ = cancel_rx.recv() => {
                    // Timer was cancelled, do nothing.
                },
            }
        });
    }
}

async fn run(mut room: Room) {
    while let Some(msg) = room.receiver.recv().await {
        room.handle_message(msg).await;
    }
}
