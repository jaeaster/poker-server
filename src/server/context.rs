use crate::*;
use std::collections::HashMap;

pub struct ConnectionInfo {
    pub ip: String,
    pub user_agent: String,
}

// Arc allows references to be shared across threads/tasks
pub struct Context {
    pub rooms: HashMap<RoomId, RoomHandle>,
    pub session: Session,
    pub connection_info: ConnectionInfo,
}
