use chashmap::{CHashMap, ReadGuard};
use std::ops::Deref;

mod models;
pub use models::*;

pub trait Storage<'a, T: 'a, P: 'a>
where
    T: Deref<Target = Table>,
    P: Deref<Target = Player>,
{
    fn get_table(&'a self, id: &str) -> T;
    fn write_table(&'a self, table: Table);

    fn get_player(&'a self, id: &str) -> P;
    fn write_player(&'a self, player: Player);
}

pub struct MemoryStore {
    pub tables: CHashMap<String, Table>,
    pub players: CHashMap<String, Player>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            tables: CHashMap::new(),
            players: CHashMap::new(),
        }
    }
}

impl<'a> Storage<'a, ReadGuard<'a, String, Table>, ReadGuard<'a, String, Player>> for MemoryStore {
    fn get_table(&'a self, id: &str) -> ReadGuard<'a, String, Table> {
        self.tables.get(id).expect("Table doesn't exist")
    }

    fn write_table(&'a self, table: Table) {
        self.tables.insert(table.id.clone(), table);
    }

    fn get_player(&'a self, id: &str) -> ReadGuard<'a, String, Player> {
        self.players.get(id).expect("Player doesn't exist")
    }

    fn write_player(&'a self, player: Player) {
        self.players.insert(player.id.clone(), player);
    }
}
