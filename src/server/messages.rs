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
    PlayerAction(PlayerEvent),
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
    GameUpdate(GameEvent),
    PlayerAction(PlayerEvent),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum GameEvent {
    NewGame {
        id: GameId,
        players: Vec<Player>,
        dealer_idx: usize,
        stacks: Vec<i32>,
        bets: Vec<i32>,
        min_raise: i32,
        bet: i32,
        current_player_idx: usize,
    },
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
        PokerMessage::Room(RoomWrapper {
            room_id: room_id.clone(),
            payload: RoomMessage::GameUpdate(GameEvent::NewGame {
                id: new_game.id.clone(),
                players: new_game.players.clone(),
                dealer_idx: state.dealer_idx,
                stacks: state.stacks.clone(),
                bets: round.player_bet.clone(),
                min_raise: round.min_raise,
                bet: round.bet,
                current_player_idx: round.to_act_idx,
            }),
        })
    }

    pub fn deal_hand(room_id: &RoomId, hand: Hand) -> Self {
        PokerMessage::Room(RoomWrapper {
            room_id: room_id.clone(),
            payload: RoomMessage::GameUpdate(GameEvent::DealHand(hand)),
        })
    }

    pub fn community_cards(
        room_id: &RoomId,
        flop: (Card, Card, Card),
        turn: Option<Card>,
        river: Option<Card>,
    ) -> Self {
        PokerMessage::Room(RoomWrapper {
            room_id: room_id.clone(),
            payload: RoomMessage::GameUpdate(GameEvent::CommunityCards { flop, turn, river }),
        })
    }
}
