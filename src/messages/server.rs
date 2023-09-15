use crate::*;
use rs_poker::core::{Card, Hand};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "messageType", content = "payload", rename_all = "camelCase")]
pub enum ServerLobby {
    TableList(Vec<TableConfig>),
    LobbyError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "messageType", content = "payload", rename_all = "camelCase")]
pub enum ServerRoomPayload {
    Chat {
        from: PlayerId,
        message: String,
    },
    SitTable {
        player: Player,
        index: usize,
    },
    RoomError(String),
    NewGame(PublicGameState),
    GameUpdate(PublicGameState),
    DealHand(Hand),
    CommunityCards {
        flop: (Card, Card, Card),
        turn: Option<Card>,
        river: Option<Card>,
    },
    DeclareWinner {
        winner: PlayerId,
        hand: Hand,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicGameState {
    pub id: GameId,
    pub players: Vec<Player>,
    pub dealer_idx: usize,
    pub game_active_players: Vec<usize>,
    pub round_active_players: Vec<usize>,
    pub current_player_idx: usize,
    pub community_cards: Vec<Card>,
    pub stacks: Vec<i32>,
    pub bets: Vec<i32>,
    pub min_raise: i32,
    pub to_call: i32,
    pub pot: i32,
}

impl PokerMessage {
    fn public_game_state_from_game(game: &Game) -> PublicGameState {
        let game_state = game.state.clone();
        let current_round = game_state.current_round_data();
        PublicGameState {
            id: game.id.clone(),
            players: game
                .players
                .clone()
                .into_iter()
                .map(GamePlayer::into)
                .collect(),
            dealer_idx: game_state.dealer_idx,
            community_cards: game_state.board.clone(),
            min_raise: current_round.min_raise,
            to_call: game.current_bet() as i32,
            current_player_idx: game.current_player_idx(),
            pot: game_state.total_pot,
            stacks: game_state.stacks.clone(),
            bets: current_round.player_bet.clone(),
            game_active_players: game_state.player_active.ones().collect(),
            round_active_players: current_round.player_active.ones().collect(),
        }
    }

    // Public methods for Lobby
    pub fn error_lobby(err: String) -> Self {
        Self::Server(Either::Lobby(ServerLobby::LobbyError(err)))
    }

    pub fn table_list(tables: Vec<TableConfig>) -> Self {
        Self::Server(Either::Lobby(ServerLobby::TableList(tables)))
    }

    // Public methods for Room
    pub fn error_room(room_id: RoomId, err: String) -> Self {
        Self::Server(Either::Room(RoomMessage {
            room_id,
            payload: ServerRoomPayload::RoomError(err),
        }))
    }

    pub fn chat_broadcast(room_id: RoomId, from: PlayerId, message: String) -> Self {
        Self::Server(Either::Room(RoomMessage {
            room_id,
            payload: ServerRoomPayload::Chat { from, message },
        }))
    }

    pub fn sit_table_broadcast(room_id: RoomId, player: Player, index: usize) -> Self {
        Self::Server(Either::Room(RoomMessage {
            room_id,
            payload: ServerRoomPayload::SitTable { player, index },
        }))
    }

    pub fn new_game(room_id: RoomId, new_game: &Game) -> Self {
        let state = Self::public_game_state_from_game(new_game);
        Self::Server(Either::Room(RoomMessage {
            room_id,
            payload: ServerRoomPayload::NewGame(state),
        }))
    }

    pub fn deal_hand(room_id: RoomId, hand: Hand) -> Self {
        Self::Server(Either::Room(RoomMessage {
            room_id,
            payload: ServerRoomPayload::DealHand(hand),
        }))
    }

    pub fn community_cards(
        room_id: RoomId,
        flop: (Card, Card, Card),
        turn: Option<Card>,
        river: Option<Card>,
    ) -> Self {
        Self::Server(Either::Room(RoomMessage {
            room_id,
            payload: ServerRoomPayload::CommunityCards { flop, turn, river },
        }))
    }

    pub fn game_update(room_id: RoomId, game: &Game) -> Self {
        let state_update = Self::public_game_state_from_game(game);
        Self::Server(Either::Room(RoomMessage {
            room_id,
            payload: ServerRoomPayload::GameUpdate(state_update),
        }))
    }
}
