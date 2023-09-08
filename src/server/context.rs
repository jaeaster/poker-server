use super::*;
use std::collections::HashMap;

pub struct ConnectionInfo {
    pub ip: String,
    pub user_agent: String,
}

// Arc allows references to be shared across threads/tasks
pub struct Context {
    pub rooms: HashMap<RoomId, RoomHandle>,
    pub session: Arc<Session>,
    pub connection_info: Arc<ConnectionInfo>,
}

impl Clone for Context {
    // Increment reference counters when cloning
    fn clone(&self) -> Self {
        Context {
            rooms: self.rooms.clone(),
            session: self.session.clone(),
            connection_info: self.connection_info.clone(),
        }
    }
}
