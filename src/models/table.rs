use super::PlayerId;
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

pub type ChipInt = u64;
pub type TableId = String;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Table {
    pub id: TableId,
    pub name: String,
    pub min_players: u8,
    pub max_players: u8,
    pub small_blind: ChipInt,
    pub big_blind: ChipInt,

    pub players: Vec<(PlayerId, ChipInt)>,
}

impl Table {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            min_players: 2,
            max_players: 9,
            small_blind: 1,
            big_blind: 2,
            players: vec![],
        }
    }

    pub fn set_blinds(&mut self, small_blind: ChipInt, big_blind: ChipInt) {
        self.small_blind = small_blind;
        self.big_blind = big_blind;
    }
}

impl Default for Table {
    fn default() -> Self {
        let player_id = Address::default();
        let mut table = Table::new(69420.to_string(), "Pocket Rocket Dreams".to_string());
        table.players = (0..table.max_players)
            .map(|_| (player_id.to_string(), 100))
            .collect();
        table
    }
}
