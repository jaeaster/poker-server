use super::messages::*;
use crate::server::{cookie::Session, ConnectionInfo, Context, GlobalState, RoomId};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    response::IntoResponse,
    TypedHeader,
};
use eyre::{eyre, Result};
use futures::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<GlobalState>>,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    cookies: Option<TypedHeader<headers::Cookie>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };

    debug!("`{}` at {} connected.", user_agent, addr);

    let cookie_name = std::env::var("POKER_COOKIE_NAME").expect("Missing POKER_COOKIE_NAME");
    let cookie_secret =
        std::env::var("POKER_SESSION_SECRET").expect("Missing POKER_SESSION_SECRET");

    let session_cookie = if let Some(TypedHeader(cookies)) = cookies {
        cookies.get(&cookie_name).unwrap().to_string()
    } else {
        String::from("cookie not found")
    };

    let session = Session::from_cookie(&session_cookie, &cookie_secret).unwrap();
    let ctx = Context {
        state,
        session: Arc::new(session),
        connection_info: Arc::new(ConnectionInfo {
            user_agent,
            ip: addr.to_string(),
        }),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, ctx))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(socket: WebSocket, ctx: Context) {
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
