use crate::*;

pub type PlayerId = String;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Player {
    pub id: PlayerId,
    pub username: String,
    pub chips: ChipInt,
}

impl Player {
    pub fn new(id: String, username: String, chips: ChipInt) -> Self {
        Self {
            id,
            username,
            chips,
        }
    }
}
