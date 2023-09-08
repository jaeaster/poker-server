use super::messages::*;
use crate::{actors::player::PlayerHandle, models::Player, server::Context};
use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{debug, error};

/// Actual websocket statemachine (one will be spawned per connection)
pub async fn handle_socket(socket: WebSocket, ctx: Context) {
    let (mut tx, mut rx) = socket.split();
    let (player_send, mut player_recv) = mpsc::channel::<PokerMessage>(8);

    let player = PlayerHandle::new(
        Player::new(
            ctx.session.address.to_string(),
            ctx.session.address.to_string(),
        ),
        ctx.rooms,
        player_send.clone(),
    );

    debug!("Registered socket for {}", &ctx.session.address);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                // Process player's messages
                Some(Ok(msg)) = rx.next() => {
                    match msg {
                        Message::Text(text) => {
                            debug!("Received message from client: {}", &text);
                            let result = serde_json::from_str::<PokerMessage>(&text);
                            match result {
                                Ok(poker_msg) => {
                                    if player.send_message(poker_msg).is_err() {
                                        error!("Socket overwhelmed; dropping connection");
                                        break;
                                    };
                                },
                                Err(_) => {
                                    let _ = tx.send(Message::Text(json!({"error": "Invalid Message"}).to_string())).await;
                                },
                            }

                        },
                        _ => unimplemented!()
                    }
                },
                // recv from player's channel and send to their socket
                Some(msg) = player_recv.recv() => {
                    if let Err(e) = tx.send(Message::Text(serde_json::to_string(&msg).unwrap())).await {
                        error!("Sending to player socket failed");
                    }
                }
            }
        }
    });
}
