use super::*;
use eyre::{eyre, Result, WrapErr};
use tracing::info;

pub type RoomId = String;
pub type PlayerId = String;
type PlayerChannel = mpsc::UnboundedSender<String>;

type Room = Vec<PlayerId>;
type Rooms = CHashMap<RoomId, Room>;
type Sockets = CHashMap<PlayerId, PlayerChannel>;

// Ensure all state is thread-safe since it will be shared
pub struct GlobalState {
    pub sockets: Sockets,
    pub rooms: Rooms,
    pub storage: MemoryStore,
}

pub struct ConnectionInfo {
    pub ip: String,
    pub user_agent: String,
}

// Arc allows references to be shared across threads/tasks
pub struct Context {
    pub state: Arc<GlobalState>,
    pub session: Arc<Session>,
    pub connection_info: Arc<ConnectionInfo>,
}

impl Clone for Context {
    // Increment reference counters when cloning
    fn clone(&self) -> Self {
        Context {
            state: self.state.clone(),
            session: self.session.clone(),
            connection_info: self.connection_info.clone(),
        }
    }
}

impl Context {
    pub fn broadcast(self, room_id: RoomId, msg: String) -> Result<()> {
        let player_ids = self
            .state
            .rooms
            .get(&room_id)
            .ok_or(eyre!("Not a valid room id"))?;

        for id in player_ids.iter() {
            if let Some(chan) = self.state.sockets.get(id) {
                info!("Sending message to {}", id.to_string());
                chan.send(msg.clone())
                    .wrap_err("Sending to player channel failed")?;
            }
        }
        Ok(())
    }

    pub fn send_to_player(self, msg: String) -> Result<()> {
        let player_send = self
            .state
            .sockets
            .get_mut(&self.session.address.to_string())
            .expect("Player has no channel");

        // Send error to player
        player_send
            .send(msg)
            .expect("Send to player channel failed");

        Ok(())
    }
}
