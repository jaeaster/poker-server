use super::messages::*;
use crate::server::{Context, RoomId};
use axum::extract::ws::{Message, WebSocket};
use eyre::{eyre, Result};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Actual websocket statemachine (one will be spawned per connection)
pub async fn handle_socket(socket: WebSocket, ctx: Context) {
    let (mut tx, mut rx) = socket.split();
    let (player_send, mut player_recv) = mpsc::unbounded_channel::<String>();

    // Register channel to send messages to player
    ctx.state
        .sockets
        .insert(ctx.session.address.to_string(), player_send);

    debug!("Registered socket for {}", &ctx.session.address);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                // Process player's messages
                Some(Ok(msg)) = rx.next() => {
                    match msg {
                        Message::Text(text) => {
                            info!("Received message from client: {}", &text);
                            let result = serde_json::from_str::<PokerMessage>(&text);
                            match result {
                                Ok(poker_msg) => handle_poker_message(poker_msg, text, ctx.clone()).await,
                                Err(e) => handle_error_response(eyre!("Invalid Message Content"), ctx.clone()).await,

                            }

                        },
                        _ => unimplemented!()
                    }
                },
                // recv from player's channel and send to their socket
                Some(msg) = player_recv.recv() => {
                    if let Err(e) = tx.send(Message::Text(msg)).await {
                        error!("Sending to player socket failed");
                    }
                }
            }
        }
    });
}

async fn handle_poker_message(msg: PokerMessage, raw_msg: String, ctx: Context) {
    let result = match msg {
        PokerMessage::Lobby(msg) => handle_lobby_message(msg, ctx.clone()).await,
        PokerMessage::Room(RoomWrapper { room_id, payload }) => {
            handle_room_message(room_id, payload, raw_msg, ctx.clone()).await
        }
    };

    if let Err(e) = result {
        handle_error_response(e, ctx).await;
    }
}

async fn handle_error_response(err: eyre::Error, ctx: Context) {
    error!("{}", err.to_string());
    let _ = ctx.send_to_player(err.to_string());
}

async fn handle_room_message(
    room_id: RoomId,
    msg: RoomMessage,
    raw_msg: String,
    ctx: Context,
) -> Result<()> {
    info!("Handling message for room {}: {}", room_id, raw_msg);
    ctx.state
        .rooms
        .get_mut(&room_id)
        .ok_or(eyre!("Not a valid room id"))?
        .push(ctx.session.address.to_string());

    match msg {
        RoomMessage::Chat(_) => ctx.broadcast(room_id, raw_msg)?,
        _ => unimplemented!(),
    }
    Ok(())
}

async fn handle_lobby_message(msg: LobbyMessage, _ctx: Context) -> Result<()> {
    match msg {
        LobbyMessage::GetTables => unimplemented!(),
    }
}
