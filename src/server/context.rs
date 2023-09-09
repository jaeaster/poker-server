use crate::*;

pub struct ConnectionInfo {
    pub ip: String,
    pub user_agent: String,
}

pub struct Context {
    pub session: Session,
    pub connection_info: ConnectionInfo,
}
