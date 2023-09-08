use serde::{Deserialize, Serialize};

pub type PlayerId = String;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Player {
    pub id: PlayerId,
    pub username: String,
}

impl Player {
    pub fn new(id: String, username: String) -> Self {
        Self { id, username }
    }
}
