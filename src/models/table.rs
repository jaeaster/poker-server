use crate::*;

pub type ChipInt = u64;
pub type TableId = RoomId;

#[derive(Debug)]
pub struct Table {
    pub config: TableConfig,
    pub players: Vec<TablePlayer>,
    pub game: Option<Game>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct TableConfig {
    pub id: TableId,
    pub name: String,
    pub min_players: usize,
    pub max_players: usize,
    pub small_blind: ChipInt,
    pub big_blind: ChipInt,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct TablePlayer {
    pub info: Player,
    pub wait_for_big_blind: bool,
    pub sit_out_next_hand: bool,
    pub sit_out_next_big_blind: bool,
}

impl TablePlayer {
    fn new(player: Player) -> Self {
        Self {
            info: player,
            wait_for_big_blind: true,
            sit_out_next_hand: false,
            sit_out_next_big_blind: false,
        }
    }
}

impl From<Player> for TablePlayer {
    fn from(player: Player) -> Self {
        Self::new(player)
    }
}

impl Table {
    pub fn new(id: String, name: String) -> Self {
        Self {
            config: TableConfig {
                id,
                name,
                min_players: 2,
                max_players: 9,
                small_blind: 1,
                big_blind: 2,
            },
            players: vec![],
            game: None,
        }
    }

    pub fn id(&self) -> &TableId {
        &self.config.id
    }

    pub fn num_players(&self) -> usize {
        self.players.len()
    }

    pub fn min_players(&self) -> usize {
        self.config.min_players
    }

    pub fn max_players(&self) -> usize {
        self.config.max_players
    }

    pub fn big_blind(&self) -> ChipInt {
        self.config.big_blind
    }

    pub fn small_blind(&self) -> ChipInt {
        self.config.small_blind
    }

    pub fn game(&self) -> Option<&Game> {
        self.game.as_ref()
    }

    pub fn game_mut(&mut self) -> Option<&mut Game> {
        self.game.as_mut()
    }

    pub fn current_player(&self) -> Option<&Player> {
        if let Some(game) = &self.game {
            Some(game.current_player())
        } else {
            None
        }
    }

    pub fn set_blinds(&mut self, small_blind: ChipInt, big_blind: ChipInt) {
        self.config.small_blind = small_blind;
        self.config.big_blind = big_blind;
    }

    pub fn set_wait_for_big_blind(&mut self, player: &Player, value: bool) -> Result<()> {
        if let Some(idx) = self.players.iter().position(|p| p.info.id == player.id) {
            self.players.get_mut(idx).unwrap().wait_for_big_blind = value;
            Ok(())
        } else {
            bail!("Player not found")
        }
    }
    pub fn set_sit_out_next_hand(&mut self, player: &Player, value: bool) -> Result<()> {
        if let Some(idx) = self.players.iter().position(|p| p.info.id == player.id) {
            self.players.get_mut(idx).unwrap().sit_out_next_hand = value;
            Ok(())
        } else {
            bail!("Player not found")
        }
    }

    pub fn set_sit_out_next_big_blind(&mut self, player: &Player, value: bool) -> Result<()> {
        if let Some(idx) = self.players.iter().position(|p| p.info.id == player.id) {
            self.players.get_mut(idx).unwrap().sit_out_next_big_blind = value;
            Ok(())
        } else {
            bail!("Player not found")
        }
    }

    pub fn set_check_fold(&mut self, player: &Player, value: bool) -> Result<()> {
        if let Some(game) = self.game_mut() {
            if let Some(idx) = game.players.iter().position(|p| p.info.id == player.id) {
                game.players.get_mut(idx).unwrap().check_fold = value;
                Ok(())
            } else {
                bail!("Player not found")
            }
        } else {
            bail!("Game is not active")
        }
    }

    pub fn set_call_any(&mut self, player: &Player, value: bool) -> Result<()> {
        if let Some(game) = self.game_mut() {
            if let Some(idx) = game.players.iter().position(|p| p.info.id == player.id) {
                game.players.get_mut(idx).unwrap().call_any = value;
                Ok(())
            } else {
                bail!("Player not found")
            }
        } else {
            bail!("Game is not active")
        }
    }

    pub fn start_new_game(&mut self) -> Result<()> {
        let players = self.get_players_for_next_game();
        if players.len() < self.min_players() {
            bail!("Not enough players to start game");
        }
        let new_game = Game::new(
            self.id().clone(),
            players,
            self.get_next_dealer_idx(),
            self.small_blind(),
            self.big_blind(),
        );

        self.game = Some(new_game);
        Ok(())
    }

    fn get_next_dealer_idx(&self) -> usize {
        self.game
            .as_ref()
            .map_or(0, |game| (game.state.dealer_idx + 1) % self.num_players())
    }

    fn get_players_for_next_game(&self) -> Vec<GamePlayer> {
        let ids_in_last_game = self.game().map_or(vec![], |g| {
            g.players.iter().map(|p| p.info.id.clone()).collect()
        });

        // TODO: Filter players sitting out next big blind
        let players_from_last_game: Vec<_> = self
            .players
            .clone()
            .into_iter()
            .filter(|p| ids_in_last_game.contains(&p.info.id))
            .filter(|p| !p.sit_out_next_hand)
            .map(GamePlayer::from)
            .collect();

        // TODO: Filter players waiting for big blind
        let new_players: Vec<_> = self
            .players
            .clone()
            .into_iter()
            .filter(|p| !ids_in_last_game.contains(&p.info.id))
            .map(GamePlayer::from)
            .collect();

        players_from_last_game
            .into_iter()
            .chain(new_players)
            .collect()
    }
}

impl Default for Table {
    fn default() -> Self {
        Table::new(69420.to_string(), "Pocket Rocket Dreams".to_string())
    }
}
