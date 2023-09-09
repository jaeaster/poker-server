use crate::*;
use axum::extract::ws::{close_code, CloseFrame, Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use futures_util::stream::SplitSink;
use std::borrow::Cow;
use tokio::sync::mpsc;

/// Actual websocket statemachine (one will be spawned per connection)
pub async fn handle_socket(socket: WebSocket, app_state: AppState, ctx: Context) {
    let (mut tx, mut rx) = socket.split();
    let (player_send, mut player_recv) = mpsc::channel::<PokerMessage>(*CHANNEL_SIZE);

    // Spawn new Player actor
    let player = PlayerHandle::new(
        Player::new(
            ctx.session.address.to_string(),
            ctx.session.address.to_string(),
            *DEFAULT_CHIPS,
        ),
        app_state.room_registry.clone(),
        player_send.clone(),
    );

    // Add to player to registry actor
    // TODO: Do we need to check for error and close socket here if this fails ?
    app_state
        .player_registry
        .set(player.id.clone(), player.clone())
        .await;

    debug!(id = ?ctx.session.address, "Registered player socket");

    tokio::spawn(async move {
        loop {
            tokio::select! {
                // Process websocket messages from player
                Some(Ok(msg)) = rx.next() => {
                    if handle_recv(msg, &player, &app_state).await.is_err() {
                        break;
                    }
                },
                // recv messages from server and forward to client
                Some(msg) = player_recv.recv() => {
                    if handle_send(msg, &mut tx).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
}

async fn handle_recv(msg: Message, player: &PlayerHandle, app_state: &AppState) -> Result<()> {
    match msg {
        Message::Text(text) => {
            debug!("Received message from client: {}", &text);
            let result = serde_json::from_str::<PokerMessage>(&text);
            match result {
                Ok(poker_msg) => {
                    if player.send_message(poker_msg).is_err() {
                        bail!("Socket overwhelmed; dropping connection");
                    };
                    Ok(())
                }
                Err(_) => {
                    let _ = player.send_error(eyre!("Invalid Message".to_owned()));
                    Ok(())
                }
            }
        }
        // Remove player from registry when their connection closes
        // `break` to close the connection server side
        Message::Close(_) => {
            app_state.player_registry.delete(player.id.clone()).await;
            bail!("Received Message::Close, dropping connection");
        }
        _ => unimplemented!(),
    }
}

async fn handle_send(msg: PokerMessage, tx: &mut SplitSink<WebSocket, Message>) -> Result<()> {
    if let Err(e) = tx
        .send(Message::Text(serde_json::to_string(&msg).unwrap()))
        .await
    {
        error!("Sending to player socket failed");
        let close_msg = Some(CloseFrame {
            code: close_code::NORMAL,
            reason: Cow::from("Goodbye"),
        });
        let _ = tx.send(Message::Close(close_msg)).await;
        bail!("Closed socket");
    }
    Ok(())
}
