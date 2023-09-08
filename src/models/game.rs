use crate::*;
use rs_poker::arena::{game_state::Round, GameState};
use rs_poker::core::{FlatDeck, Hand, Rank, Rankable};

pub type GameId = TableId;

pub struct Game {
    pub id: GameId,
    pub players: Vec<Player>,
    pub state: GameState,
    pub deck: FlatDeck,
}

impl Game {
    pub fn new(
        id: GameId,
        players: Vec<Player>,
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
            players.iter().map(|player| player.chips as i32).collect(),
            big_blind as i32,
            small_blind as i32,
            dealer_idx,
        );

        game_state.hands = hands;

        Self {
            id,
            players,
            deck,
            state: game_state,
        }
    }

    pub fn bet(&mut self, amount: ChipInt) -> Result<i32, rs_poker::arena::errors::GameStateError> {
        self.state.do_bet(amount as i32, false)
    }

    pub fn fold(&mut self) {
        self.state.fold()
    }

    pub fn advance(&mut self) {
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

    pub fn is_complete(&self) -> bool {
        self.state.is_complete()
    }

    pub fn complete(&mut self) {
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
        let player = Player::new(player_id, username, *DEFAULT_CHIPS);
        table.players = (0..table.max_players).map(|_| player.clone()).collect();
        let dealer_idx = 0;
        let mut game = Game::new(
            table.id,
            table.players.clone(),
            dealer_idx,
            table.small_blind,
            table.big_blind,
        );

        // Advance from start -> preflop state and take the blinds
        game.advance();
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
        game.advance();

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
        game.advance();

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
        game.advance();

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
        game.advance();
        // Rank hands, determine winner(s), update chips
        // Showdown -> Complete
        game.complete();

        Ok(())
    }
}
