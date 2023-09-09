use crate::*;
use std::collections::HashMap;
use tokio::sync::mpsc;

enum PlayerRegistryMessage {
    Get(PlayerId, mpsc::Sender<Option<PlayerHandle>>),
    Set(PlayerId, PlayerHandle),
    Delete(PlayerId),
}

#[derive(Clone)]
pub struct PlayerRegistryHandle {
    sender: mpsc::Sender<PlayerRegistryMessage>,
}

impl PlayerRegistryHandle {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(*CHANNEL_SIZE);
        let registry = PlayerRegistryActor::new(HashMap::new(), receiver);
        tokio::spawn(run(registry));

        Self { sender }
    }

    pub async fn get(&self, player_id: PlayerId) -> Option<PlayerHandle> {
        let (response_tx, mut response_rx) = mpsc::channel(1);
        let _ = self
            .sender
            .send(PlayerRegistryMessage::Get(player_id, response_tx))
            .await;
        response_rx.recv().await.unwrap_or(None)
    }

    pub async fn set(&self, player_id: PlayerId, player_handle: PlayerHandle) {
        let _ = self
            .sender
            .send(PlayerRegistryMessage::Set(player_id, player_handle))
            .await;
    }

    pub async fn delete(&self, player_id: PlayerId) {
        let _ = self
            .sender
            .send(PlayerRegistryMessage::Delete(player_id))
            .await;
    }
}

struct PlayerRegistryActor {
    players: HashMap<PlayerId, PlayerHandle>,
    receiver: mpsc::Receiver<PlayerRegistryMessage>,
}

impl PlayerRegistryActor {
    pub fn new(
        players: HashMap<PlayerId, PlayerHandle>,
        receiver: mpsc::Receiver<PlayerRegistryMessage>,
    ) -> Self {
        PlayerRegistryActor { players, receiver }
    }

    async fn handle_message(&mut self, msg: PlayerRegistryMessage) {
        match msg {
            PlayerRegistryMessage::Get(player_id, sender) => {
                let player_handle = self.players.get(&player_id).cloned();
                let _ = sender.send(player_handle).await;
            }
            PlayerRegistryMessage::Set(player_id, player_handle) => {
                self.players.insert(player_id, player_handle);
            }
            PlayerRegistryMessage::Delete(player_id) => {
                self.players.remove(&player_id);
            }
        }
    }
}

async fn run(mut registry: PlayerRegistryActor) {
    while let Some(msg) = registry.receiver.recv().await {
        let _ = registry.handle_message(msg).await;
    }
}
