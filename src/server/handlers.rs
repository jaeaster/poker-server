use crate::messages::*;
use crate::server::{Context, GlobalState, RoomId};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    response::IntoResponse,
    TypedHeader,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<GlobalState>>,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    tracing::info!("`{}` at {} connected.", user_agent, addr);

    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(socket: WebSocket, who: SocketAddr, state: Arc<GlobalState>) {
    let (mut tx, mut rx) = socket.split();
    let (player_send, mut player_recv) = mpsc::unbounded_channel::<String>();
    let player_id = who.to_string();

    // Register channel to send messages to player
    state.sockets.insert(player_id.clone(), player_send);
    tracing::info!("Registered socket for {}", &player_id);

    let player_id = Arc::new(player_id);

    let ctx = Context { state, player_id };

    tokio::spawn(async move {
        loop {
            tokio::select! {
                // Process player's messages
                Some(Ok(msg)) = rx.next() => {
                    match msg {
                        Message::Text(text) => {
                            tracing::info!("Received message from client: {}", &text);
                            let poker_msg: PokerMessage = serde_json::from_str(&text).expect("Invalid Message");

                            match poker_msg {
                                PokerMessage::Lobby(msg) => handle_lobby_message(msg, ctx.clone()).await,
                                PokerMessage::Room(RoomWrapper { id, payload }) => handle_room_message(id, payload, text, ctx.clone()).await,
                            }
                        },
                        _ => unimplemented!()
                    }
                },
                // recv from player's channel and send to their socket
                Some(msg) = player_recv.recv() => {
                    tx.send(Message::Text(msg)).await.expect("Sending to player socket failed");
                }
            }
        }
    });
}

async fn handle_room_message(room_id: RoomId, msg: RoomMessage, raw_msg: String, ctx: Context) {
    tracing::info!("Handling message for room {}: {}", room_id, raw_msg);
    ctx.state
        .rooms
        .get_mut(&room_id)
        .expect("Not a valid room")
        .push(ctx.player_id.to_string());
    if let Some(player_ids) = ctx.state.rooms.get(&room_id) {
        match msg {
            RoomMessage::Chat(_) => {
                for id in player_ids.iter() {
                    if let Some(chan) = ctx.state.sockets.get(id) {
                        tracing::info!("Sending message to {}", id.to_string());
                        chan.send(raw_msg.clone())
                            .expect("Sending to player channel failed");
                    }
                }
            }
            _ => unimplemented!(),
        }
    }
}

async fn handle_lobby_message(_msg: LobbyMessage, _ctx: Context) {
    unimplemented!();
}
