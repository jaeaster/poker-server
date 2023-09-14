use crate::*;
use rs_poker::arena::{game_state::Round, GameState};
use rs_poker::core::{FlatDeck, Hand, Rank, Rankable};

pub type GameId = TableId;

#[derive(Debug)]
pub struct Game {
    pub id: GameId,
    pub players: Vec<GamePlayer>,
    pub state: GameState,
    pub deck: FlatDeck,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GamePlayer {
    pub info: Player,
    check_fold: bool,
    call_any: bool,
}

impl GamePlayer {
    fn new(player: Player) -> Self {
        Self {
            info: player,
            check_fold: false,
            call_any: false,
        }
    }
}

impl From<Player> for GamePlayer {
    fn from(player: Player) -> Self {
        Self::new(player)
    }
}

impl From<TablePlayer> for GamePlayer {
    fn from(player: TablePlayer) -> Self {
        Self::new(player.info)
    }
}

impl Game {
    pub fn new(
        id: GameId,
        players: Vec<GamePlayer>,
        dealer_idx: usize,
        small_blind: ChipInt,
        big_blind: ChipInt,
    ) -> Self {
        // shuffled deck of 52 cards
        let mut deck = FlatDeck::default();
        let mut hands: Vec<Hand> = (0..players.len()).map(|_| Hand::default()).collect();

        // Deal 2 cards to each player
        for _ in 0..2 {
            for hand in &mut hands {
                let next_card = deck.deal().expect("Deck should not be empty");
                hand.push(next_card);
            }
        }
        debug!("Players hands: {:?}", &hands);

        let mut game_state = GameState::new(
            (0..players.len()).map(|_| *DEFAULT_CHIPS as i32).collect(),
            big_blind as i32,
            small_blind as i32,
            dealer_idx,
        );

        game_state.hands = hands;

        let mut new_game = Self {
            id,
            players,
            deck,
            state: game_state,
        };

        // Advance to preflop and take blinds
        new_game.advance_round();
        new_game
    }

    pub fn players_hands(&self) -> Vec<(&Player, &Hand)> {
        self.players
            .iter()
            .map(|p| &p.info)
            .zip(self.state.hands.iter())
            .collect()
    }

    pub fn current_player_idx(&self) -> usize {
        self.state.current_round_data().to_act_idx
    }

    pub fn current_player(&self) -> &Player {
        &self
            .players
            .get(self.current_player_idx())
            .expect("Current player should exist")
            .info
    }

    pub fn is_players_turn(&self, player: &Player) -> bool {
        self.current_player().id == player.id
    }

    pub fn current_bet(&self) -> ChipInt {
        self.state.current_round_data().bet as ChipInt
    }

    pub fn players_bet(&self, player_idx: usize) -> ChipInt {
        self.state.current_round_data().player_bet[player_idx] as ChipInt
    }

    pub fn bet(&mut self, amount: ChipInt) -> Result<i32, rs_poker::arena::errors::GameStateError> {
        let bet = self.state.do_bet(amount as i32, false)?;
        self.advance();
        Ok(bet)
    }

    pub fn fold(&mut self) {
        self.state.fold();
        self.advance();
    }

    pub fn is_over(&self) -> bool {
        self.state.round == Round::Complete
    }

    fn check_fold(&mut self) {
        if self.players_bet(self.current_player_idx()) == self.current_bet() {
            self.bet(self.current_bet())
                .expect("Check / Fold should work");
        } else {
            self.fold();
        }
    }

    fn call_any(&mut self) {
        let current_bet = self.current_bet() as i32;
        self.bet(current_bet as ChipInt)
            .expect("Call any should be valid");
    }

    fn advance(&mut self) {
        // If last action was a fold to end the game, just complete
        if self.is_complete() {
            self.complete();
            return;
        }

        // Check if next player has an auto-action and execute it
        // This will make a recursive call back to this advance() function, therefore we return
        let current_player_idx = self.current_player_idx();
        if self
            .players
            .get(current_player_idx)
            .expect("Current player should exist")
            .check_fold
        {
            self.check_fold();
            return;
        }

        if self
            .players
            .get(current_player_idx)
            .expect("Current player should exist")
            .call_any
        {
            self.call_any();
            return;
        }

        // If last action ended the betting round, advance then check complete
        if self.state.current_round_data().player_active.empty() {
            self.advance_round();
            if self.is_complete() {
                self.complete();
            }
        }
    }

    fn advance_round(&mut self) {
        self.state.advance_round();

        match self.state.round {
            Round::Flop => {
                self.state.board = (0..3)
                    .map(|_| self.deck.deal().expect("Deck should not be empty"))
                    .collect()
            }
            Round::Turn | Round::River => self
                .state
                .board
                .push(self.deck.deal().expect("Deck should not be empty")),
            _ => (),
        }
    }

    fn is_complete(&self) -> bool {
        self.state.is_complete()
    }

    fn complete(&mut self) {
        self.state.complete();
        match self.state.num_active_players() {
            0 => panic!("No active players when game is complete"),
            1 => {
                if let Some(winner_idx) = self.state.player_active.ones().next() {
                    self.state.award(winner_idx, self.state.total_pot)
                }
            }
            _ => {
                let ranks = self.rank_active_players();
                debug!("Community Cards {:?}", self.state.board);
                debug!("Players Ranks: {:?}", ranks);
                // TODO: Handle ties + side pots
                self.state.award(
                    ranks.first().expect("Should be at least 2 players").1,
                    self.state.total_pot,
                )
            }
        }
    }

    fn rank_active_players(&self) -> Vec<(Rank, usize)> {
        let mut ranks = self
            .state
            .player_active
            .ones()
            .map(|idx| {
                let mut hand = self
                    .state
                    .hands
                    .get(idx)
                    .expect("Player should have a hand")
                    .clone();

                // Add community cards
                hand.extend(self.state.board.clone());
                hand.rank()
            })
            .zip(self.state.player_active.ones())
            .collect::<Vec<_>>();
        ranks.sort_unstable_by(|r1, r2| r2.0.cmp(&r1.0));

        ranks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_game() -> Result<()> {
        let mut table = Table::default();
        let player_id = Address::default().to_string();
        let username = player_id.clone();
        let player = Player::new(player_id, username);
        table.players = (0..table.max_players())
            .map(|_| player.clone().into())
            .collect();
        let dealer_idx = 0;
        let mut game = Game::new(
            table.id().clone(),
            table
                .players
                .clone()
                .into_iter()
                .map(GamePlayer::from)
                .collect(),
            dealer_idx,
            table.small_blind(),
            table.big_blind(),
        );

        // Advance from start -> preflop state and take the blinds
        game.advance_round();
        // First to bet is UTG, everyone calls
        for _ in 0..game.players.len() {
            let round_data = game.state.current_round_data();
            let player_idx = round_data.to_act_idx;
            let bet = game.state.do_bet(game.state.big_blind, false)?;
            debug!("Player at {} bet {}", player_idx, bet);

            if game.is_complete() {
                game.complete();
                return Ok(());
            }
        }

        // Preflop -> Flop
        game.advance_round();

        // BB first to bet, everyone checks
        for _ in 0..game.players.len() {
            let round_data = game.state.current_round_data();
            let player_idx = round_data.to_act_idx;
            let bet = game.state.do_bet(0, false)?;
            info!("Player {} bet {}", player_idx, bet);

            if game.is_complete() {
                game.complete();
                return Ok(());
            }
        }

        // Flop -> Turn
        game.advance_round();

        // Everyone checks
        for _ in 0..game.players.len() {
            let round_data = game.state.current_round_data();
            let player_idx = round_data.to_act_idx;
            let bet = game.state.do_bet(0, false)?;
            info!("Player {} bet {}", player_idx, bet);

            if game.is_complete() {
                game.complete();
                return Ok(());
            }
        }

        // Turn -> River
        game.advance_round();

        // Final round of betting, Everyone checks
        for _ in 0..game.players.len() {
            let round_data = game.state.current_round_data();
            let player_idx = round_data.to_act_idx;
            let bet = game.state.do_bet(0, false)?;
            info!("Player {} bet {}", player_idx, bet);

            if game.is_complete() {
                game.complete();
                return Ok(());
            }
        }

        // River -> Showdown
        game.advance_round();
        // Rank hands, determine winner(s), update chips
        // Showdown -> Complete
        game.complete();

        Ok(())
    }
}
