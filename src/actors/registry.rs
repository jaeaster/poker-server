use crate::*;
use std::collections::HashMap;
use std::hash::Hash;
use tokio::sync::mpsc;

// Generic over actor id, actor handle
enum RegistryMessage<ID, Handle>
where
    ID: Eq + Hash + Send + 'static,
    Handle: Clone + Send + 'static,
{
    Get(ID, mpsc::Sender<Option<Handle>>),
    GetAll(mpsc::Sender<Vec<Handle>>),
    Set(ID, Handle),
    Delete(ID),
}

#[derive(Clone)]
pub struct RegistryHandle<ID, Handle>
where
    ID: Eq + Hash + Send + 'static,
    Handle: Clone + Send + 'static,
{
    sender: mpsc::Sender<RegistryMessage<ID, Handle>>,
}

impl<ID, Handle> RegistryHandle<ID, Handle>
where
    ID: Eq + Hash + Send + 'static,
    Handle: Clone + Send + 'static,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(*CHANNEL_SIZE);
        let registry = RegistryActor::new(HashMap::new(), receiver);
        tokio::spawn(run(registry));

        Self { sender }
    }

    pub async fn get(&self, id: ID) -> Option<Handle> {
        let (response_tx, mut response_rx) = mpsc::channel(1);
        let _ = self
            .sender
            .send(RegistryMessage::Get(id, response_tx))
            .await;
        response_rx.recv().await.unwrap_or(None)
    }

    pub async fn get_all(&self) -> Vec<Handle> {
        let (response_tx, mut response_rx) = mpsc::channel(1);
        let _ = self.sender.send(RegistryMessage::GetAll(response_tx)).await;
        response_rx.recv().await.expect("Registry Actor Died")
    }

    pub async fn set(&self, id: ID, handle: Handle) {
        let _ = self.sender.send(RegistryMessage::Set(id, handle)).await;
    }

    pub async fn delete(&self, id: ID) {
        let _ = self.sender.send(RegistryMessage::Delete(id)).await;
    }
}

struct RegistryActor<ID, Handle>
where
    ID: Eq + Hash + Send + 'static,
    Handle: Clone + Send + 'static,
{
    registry: HashMap<ID, Handle>,
    receiver: mpsc::Receiver<RegistryMessage<ID, Handle>>,
}

impl<ID, Handle> RegistryActor<ID, Handle>
where
    ID: Eq + Hash + Send + 'static,
    Handle: Clone + Send + 'static,
{
    pub fn new(
        registry: HashMap<ID, Handle>,
        receiver: mpsc::Receiver<RegistryMessage<ID, Handle>>,
    ) -> Self {
        Self { registry, receiver }
    }

    async fn handle_message(&mut self, msg: RegistryMessage<ID, Handle>) {
        match msg {
            RegistryMessage::Get(id, sender) => {
                let handle = self.registry.get(&id).cloned();
                let _ = sender.send(handle).await;
            }
            RegistryMessage::GetAll(sender) => {
                let handles = self.registry.values().cloned().collect();
                let _ = sender.send(handles).await;
            }
            RegistryMessage::Set(id, handle) => {
                self.registry.insert(id, handle);
            }
            RegistryMessage::Delete(id) => {
                self.registry.remove(&id);
            }
        }
    }
}

async fn run<ID, Handle>(mut registry: RegistryActor<ID, Handle>)
where
    ID: Eq + Hash + Send + 'static,
    Handle: Clone + Send + 'static,
{
    while let Some(msg) = registry.receiver.recv().await {
        let _ = registry.handle_message(msg).await;
    }
}
