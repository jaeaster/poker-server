pub struct Table {
    pub id: String,
    pub name: String,
    pub min_players: u8,
    pub max_players: u8,
    pub small_blind: u64,
    pub big_blind: u64,
}

pub struct Player {
    pub id: String,
    pub username: String,
}
