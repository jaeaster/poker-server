#![allow(dead_code)]
#![allow(unused_variables)]

use dotenv::dotenv;
use lazy_static::lazy_static;
use std::env::var;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod actors;
mod models;
mod server;

pub use actors::*;
pub use alloy_primitives::Address;
pub use eyre::{bail, eyre, Result};
pub use models::*;
pub use serde::{Deserialize, Serialize};
pub use server::*;
pub use tracing::{debug, error, info, span};

lazy_static! {
    pub static ref COOKIE_NAME: String =
        var("POKER_COOKIE_NAME").expect("Missing POKER_COOKIE_NAME");
    pub static ref COOKIE_SECRET: String =
        var("POKER_SESSION_SECRET").expect("Missing POKER_SESSION_SECRET");
    pub static ref ENVIRONMENT: String = var("RUST_ENV").expect("Missing RUST_ENV");
    pub static ref ADDR: &'static str = "0.0.0.0:8080";
    pub static ref DEFAULT_CHIPS: ChipInt = 100;
    pub static ref CHANNEL_SIZE: usize = 8;
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    // Force loading of env vars
    let _ = COOKIE_NAME.clone();
    let _ = COOKIE_SECRET.clone();
    let _ = ENVIRONMENT.clone();

    // Tracing initialization
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "poker_reloaded=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    server::run().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::cookie::Session;
    use crate::server::messages::{PokerMessage, ServerMessage};
    use futures::{sink::SinkExt, stream::StreamExt};
    use test_log::test;
    use tokio::net::TcpStream;
    use tokio::task::JoinHandle;
    use tokio_tungstenite::tungstenite::handshake::client::{generate_key, Request};
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
    use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
    use tracing::debug;

    fn start_server() -> JoinHandle<()> {
        tokio::spawn(server::run())
        // tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    struct ClientConnection {
        data: Player,
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    }

    impl ClientConnection {
        async fn setup_conn() -> Self {
            let url = "ws://localhost:8080/ws";
            // Dev environment cookie
            let session = Session::default();
            let mut cookie = COOKIE_NAME.clone();
            cookie.push('=');
            cookie.push_str(&session.to_cookie(&COOKIE_SECRET));

            let req = Request::builder()
                .uri(url)
                .method("GET")
                .header("Host", url)
                .header("cookie", cookie)
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", generate_key())
                .body(())
                .unwrap();
            let (ws_stream, _) = connect_async(req).await.expect("Failed to connect");
            Self {
                data: Player::new(session.address.to_string(), session.address.to_string()),
                ws_stream,
            }
        }

        async fn get_tables(&mut self) -> Vec<Table> {
            let get_tables_msg = PokerMessage::get_tables();
            let get_tables_msg = serde_json::to_string(&get_tables_msg).unwrap();

            debug!("Sending get tables message from client");
            self.ws_stream
                .send(Message::Text(get_tables_msg))
                .await
                .expect("Failed to send message");

            if let Some(Ok(Message::Text(msg))) = self.ws_stream.next().await {
                let msg = serde_json::from_str::<PokerMessage>(&msg).unwrap();
                if let PokerMessage::ServerResponse(ServerMessage::TableList(tables)) = msg {
                    tables
                } else {
                    panic!("Received invalid get tables response");
                }
            } else {
                panic!("Didn't receive get tables response");
            }
        }

        async fn subscribe_room(&mut self, room_id: &RoomId) {
            let subscribe_msg = PokerMessage::subscribe_room(room_id.clone());
            let subscribe_msg = serde_json::to_string(&subscribe_msg).unwrap();

            debug!("Sending subscribe message from client");
            self.ws_stream
                .send(Message::Text(subscribe_msg))
                .await
                .expect("Failed to send message");
        }

        async fn send_chat(&mut self, message: &str, room_id: &RoomId) {
            let chat_msg = PokerMessage::chat(room_id.clone(), message.to_owned());
            let chat_msg = serde_json::to_string(&chat_msg).unwrap();

            debug!("Sending chat message from client");
            self.ws_stream
                .send(Message::Text(chat_msg))
                .await
                .expect("Failed to send message");
        }

        async fn sit_table(&mut self, chips: ChipInt, room_id: &RoomId) {
            let sit_msg = PokerMessage::sit_table(room_id.clone(), chips);
            let sit_msg = serde_json::to_string(&sit_msg).unwrap();

            debug!("Sending sit table from client");
            self.ws_stream
                .send(Message::Text(sit_msg))
                .await
                .expect("Failed to send message");
        }

        async fn bet(&mut self, chips: ChipInt, room_id: &RoomId) {
            let bet_msg = PokerMessage::bet(room_id, chips);
            let bet_msg = serde_json::to_string(&bet_msg).unwrap();

            debug!("Sending bet from client");
            self.ws_stream
                .send(Message::Text(bet_msg))
                .await
                .expect("Failed to send message");
        }

        async fn fold(&mut self, room_id: &RoomId) {
            let fold_msg = PokerMessage::fold(room_id);
            let fold_msg = serde_json::to_string(&fold_msg).unwrap();

            debug!("Sending bet from client");
            self.ws_stream
                .send(Message::Text(fold_msg))
                .await
                .expect("Failed to send message");
        }

        async fn receive_msg(&mut self, expected_msg: PokerMessage) {
            if let Some(msg) = self.ws_stream.next().await {
                let msg = msg.expect("Failed to read message");
                match msg {
                    Message::Text(text) => {
                        let msg = serde_json::from_str::<PokerMessage>(&text).unwrap();
                        debug!(msg = ?msg);
                        assert_eq!(msg, expected_msg);
                    }
                    _ => panic!("Received unexpected message type"),
                }
            } else {
                panic!("Did not receive a reply");
            }
        }

        async fn receive_new_game(&mut self, room_id: &RoomId) {
            if let Some(msg) = self.ws_stream.next().await {
                let msg = msg.expect("Failed to read message");
                match msg {
                    Message::Text(text) => {
                        let msg = serde_json::from_str::<PokerMessage>(&text).unwrap();
                        debug!(msg = ?msg);
                        assert!(matches!(
                            msg,
                            PokerMessage::ServerResponse(ServerMessage::GameUpdate(
                                GameEvent::NewGame(PublicGameState {
                                    id,
                                    players,
                                    dealer_idx,
                                    community_cards,
                                    stacks,
                                    bets,
                                    min_raise,
                                    to_call,
                                    current_player_idx,
                                    pot,
                                    game_active_players,
                                    round_active_players,
                                })
                            ))
                        ));
                    }
                    _ => panic!("Received unexpected message type"),
                }
            } else {
                panic!("Did not receive a reply");
            }
        }

        async fn receive_deal_hand(&mut self, room_id: &RoomId) {
            if let Some(msg) = self.ws_stream.next().await {
                let msg = msg.expect("Failed to read message");
                match msg {
                    Message::Text(text) => {
                        let msg = serde_json::from_str::<PokerMessage>(&text).unwrap();
                        debug!(msg = ?msg);
                        assert!(matches!(
                            msg,
                            PokerMessage::ServerResponse(ServerMessage::GameUpdate(
                                GameEvent::DealHand(hand)
                            ))
                        ));
                    }
                    _ => panic!("Received unexpected message type"),
                }
            } else {
                panic!("Did not receive a reply");
            }
        }

        async fn receive_game_update(&mut self, room_id: &RoomId) {
            if let Some(msg) = self.ws_stream.next().await {
                let msg = msg.expect("Failed to read message");
                match msg {
                    Message::Text(text) => {
                        let msg = serde_json::from_str::<PokerMessage>(&text).unwrap();
                        debug!(msg = ?msg);
                        assert!(matches!(
                            msg,
                            PokerMessage::ServerResponse(ServerMessage::GameUpdate(
                                GameEvent::StateUpdate(PublicGameState {
                                    id,
                                    players,
                                    dealer_idx,
                                    current_player_idx,
                                    game_active_players,
                                    round_active_players,
                                    community_cards,
                                    stacks,
                                    bets,
                                    min_raise,
                                    to_call,
                                    pot,
                                })
                            ))
                        ));
                    }
                    _ => panic!("Received unexpected message type"),
                }
            } else {
                panic!("Did not receive a reply");
            }
        }
    }

    #[test(tokio::test)]
    async fn test_get_lobby_subscribe_chat() {
        dotenv().ok();
        let server_handle = start_server();

        let mut player1 = ClientConnection::setup_conn().await;
        let tables = player1.get_tables().await;
        assert_eq!(tables.len(), 1);

        let table = tables.first().unwrap();
        let room_id = table.id.clone();

        player1.subscribe_room(&room_id).await;
        player1.send_chat("Hello, World!", &room_id).await;
        player1
            .receive_msg(PokerMessage::chat_broadcast(
                player1.data.id.clone(),
                "Hello, World!".to_owned(),
            ))
            .await;

        let mut player2 = ClientConnection::setup_conn().await;

        // Chatting
        player2.subscribe_room(&room_id).await;
        player2.send_chat("yo", &room_id).await;
        player1
            .receive_msg(PokerMessage::chat_broadcast(
                player2.data.id.clone(),
                "yo".to_owned(),
            ))
            .await;
        player2
            .receive_msg(PokerMessage::chat_broadcast(
                player2.data.id.clone(),
                "yo".to_owned(),
            ))
            .await;

        // Sitting at table
        player1.sit_table(*DEFAULT_CHIPS + 1, &room_id).await;
        player1
            .receive_msg(PokerMessage::error("Insufficient Chips".to_owned()))
            .await;

        let expected_msg = PokerMessage::sit_table_broadcast(player1.data.clone(), 0);
        player1.sit_table(*DEFAULT_CHIPS, &room_id).await;
        player2.receive_msg(expected_msg.clone()).await;
        player1.receive_msg(expected_msg).await;

        // player1.sit_table(1, &room_id).await;
        // player1
        //     .receive_msg(PokerMessage::error("Insufficient Chips".to_owned()))
        //     .await;

        let expected_msg = PokerMessage::sit_table_broadcast(player2.data.clone(), 1);
        player2.sit_table(*DEFAULT_CHIPS, &room_id).await;
        player2.receive_msg(expected_msg.clone()).await;
        player1.receive_msg(expected_msg).await;

        player1.receive_new_game(&room_id).await;
        player2.receive_new_game(&room_id).await;

        player1.receive_deal_hand(&room_id).await;
        player2.receive_deal_hand(&room_id).await;

        player1.bet(10, &room_id).await;
        player1
            .receive_msg(PokerMessage::error("Not your turn".to_owned()))
            .await;

        // Preflop
        player2.bet(2, &room_id).await;
        player2.receive_game_update(&room_id).await;
        player1.receive_game_update(&room_id).await;

        player1.bet(2, &room_id).await;
        player2.receive_game_update(&room_id).await;
        player1.receive_game_update(&room_id).await;

        // Flop
        player2.bet(0, &room_id).await;
        player2.receive_game_update(&room_id).await;
        player1.receive_game_update(&room_id).await;

        player1.bet(0, &room_id).await;
        player2.receive_game_update(&room_id).await;
        player1.receive_game_update(&room_id).await;

        // Turn
        player2.bet(0, &room_id).await;
        player2.receive_game_update(&room_id).await;
        player1.receive_game_update(&room_id).await;

        player1.bet(0, &room_id).await;
        player2.receive_game_update(&room_id).await;
        player1.receive_game_update(&room_id).await;

        // River
        player2.bet(2, &room_id).await;
        player2.receive_game_update(&room_id).await;
        player1.receive_game_update(&room_id).await;

        player1.fold(&room_id).await;
        player2.receive_game_update(&room_id).await;
        player1.receive_game_update(&room_id).await;

        // let expected_msg = PokerMessage::error("Table is full".to_string());
        server_handle.abort();
    }
}
