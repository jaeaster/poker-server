use crate::*;
use rs_poker::core::{Card, Hand};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum PokerMessage {
    Lobby(LobbyMessage),
    Room(RoomWrapper),
    ServerResponse(ServerMessage),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    TableList(Vec<Table>),
    Chat { from: PlayerId, message: String },
    SitTable { player: Player, index: usize },
    GameUpdate(GameEvent),
    Error(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum LobbyMessage {
    GetTables,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct RoomWrapper {
    pub room_id: RoomId,

    #[serde(flatten)]
    pub payload: RoomMessage,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum RoomMessage {
    Subscribe,
    Chat(String),
    SitTable { chips: ChipInt },
    PlayerAction(PlayerEvent),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum GameEvent {
    NewGame(PublicGameState),
    StateUpdate(PublicGameState),
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

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum PlayerEvent {
    Bet(ChipInt),
    Fold,
}

impl PokerMessage {
    pub fn error(err: String) -> Self {
        PokerMessage::ServerResponse(ServerMessage::Error(err))
    }

    pub fn get_tables() -> Self {
        PokerMessage::Lobby(LobbyMessage::GetTables)
    }

    pub fn table_list(tables: Vec<Table>) -> Self {
        PokerMessage::ServerResponse(ServerMessage::TableList(tables))
    }

    pub fn subscribe_room(room_id: RoomId) -> Self {
        PokerMessage::Room(RoomWrapper {
            room_id,
            payload: RoomMessage::Subscribe,
        })
    }

    pub fn chat(room_id: RoomId, message: String) -> Self {
        PokerMessage::Room(RoomWrapper {
            room_id,
            payload: RoomMessage::Chat(message),
        })
    }

    pub fn chat_broadcast(from: PlayerId, message: String) -> Self {
        PokerMessage::ServerResponse(ServerMessage::Chat { from, message })
    }

    pub fn sit_table(room_id: RoomId, chips: ChipInt) -> Self {
        PokerMessage::Room(RoomWrapper {
            room_id,
            payload: RoomMessage::SitTable { chips },
        })
    }

    pub fn sit_table_broadcast(player: Player, index: usize) -> Self {
        PokerMessage::ServerResponse(ServerMessage::SitTable { player, index })
    }

    pub fn new_game(room_id: &RoomId, new_game: &Game) -> Self {
        let state = new_game.state.clone();
        let round = state.current_round_data();
        PokerMessage::ServerResponse(ServerMessage::GameUpdate(GameEvent::NewGame(
            PublicGameState {
                id: new_game.id.clone(),
                players: new_game.players.clone(),
                dealer_idx: state.dealer_idx,
                community_cards: state.board.clone(),
                stacks: state.stacks.clone(),
                bets: round.player_bet.clone(),
                min_raise: round.min_raise,
                to_call: round.bet,
                current_player_idx: round.to_act_idx,
                pot: state.total_pot,
                game_active_players: state.player_active.ones().collect(),
                round_active_players: round.player_active.ones().collect(),
            },
        )))
    }

    pub fn deal_hand(room_id: &RoomId, hand: Hand) -> Self {
        PokerMessage::ServerResponse(ServerMessage::GameUpdate(GameEvent::DealHand(hand)))
    }

    pub fn community_cards(
        room_id: &RoomId,
        flop: (Card, Card, Card),
        turn: Option<Card>,
        river: Option<Card>,
    ) -> Self {
        PokerMessage::ServerResponse(ServerMessage::GameUpdate(GameEvent::CommunityCards {
            flop,
            turn,
            river,
        }))
    }

    pub fn bet(room_id: &RoomId, bet: ChipInt) -> Self {
        PokerMessage::Room(RoomWrapper {
            room_id: room_id.clone(),
            payload: RoomMessage::PlayerAction(PlayerEvent::Bet(bet)),
        })
    }

    pub fn fold(room_id: &RoomId) -> Self {
        PokerMessage::Room(RoomWrapper {
            room_id: room_id.clone(),
            payload: RoomMessage::PlayerAction(PlayerEvent::Fold),
        })
    }

    pub fn game_update(room_id: &RoomId, game: &Game) -> Self {
        let game_state = game.state.clone();
        let current_round = game_state.current_round_data();

        let state_update = PublicGameState {
            id: game.id.clone(),
            players: game.players.clone(),
            dealer_idx: game_state.dealer_idx,
            community_cards: game_state.board.clone(),
            min_raise: current_round.min_raise,
            to_call: current_round.bet,
            current_player_idx: current_round.to_act_idx,
            pot: game_state.total_pot,
            stacks: game_state.stacks.clone(),
            bets: game_state.player_bet.clone(),
            game_active_players: game_state.player_active.ones().collect(),
            round_active_players: current_round.player_active.ones().collect(),
        };
        PokerMessage::ServerResponse(ServerMessage::GameUpdate(GameEvent::StateUpdate(
            state_update,
        )))
    }
}
