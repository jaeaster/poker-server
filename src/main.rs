#![allow(dead_code)]
#![allow(unused_variables)]

use dotenv::dotenv;
use lazy_static::lazy_static;
use std::env::var;
use storage::{MemoryStore, Storage, Table};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod game;
mod server;
mod storage;

lazy_static! {
    pub static ref COOKIE_NAME: String =
        var("POKER_COOKIE_NAME").expect("Missing POKER_COOKIE_NAME");
    pub static ref COOKIE_SECRET: String =
        var("POKER_SESSION_SECRET").expect("Missing POKER_SESSION_SECRET");
    pub static ref ENVIRONMENT: String = var("RUST_ENV").expect("Missing RUST_ENV");
    pub static ref ADDR: &'static str = "0.0.0.0:8080";
}

// Main Entry Point
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

    let storage = MemoryStore::new();
    let table = Table::default();
    storage.write_table(table);
    server::run(storage).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{sink::SinkExt, stream::StreamExt};
    use serde_json::json;
    use test_log::test;
    use tokio::net::TcpStream;
    use tokio_tungstenite::tungstenite::handshake::client::{generate_key, Request};
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
    use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
    use tracing::info;

    async fn setup_conn() -> WebSocketStream<MaybeTlsStream<TcpStream>> {
        let url = "ws://localhost:8080/ws";
        // Dev environment cookie
        let cookie = "poker-session-dev=Fe26.2*1*8dc93bbc3f6bebfa3ff420ae8c5c7759a82b37ba3e03cd93230650157f977aa2*iaulAH2srSxJQMYMmHudVQ*tCTDJGo3SSJDbY4T2rdDnb-X6hCDNaRnK-lpOkkviQ1_gnP4ordWDtLi8WTyCcVUGvdNwGSuBx1ReNs2xMb8Z466JyPlmmQvIDApwlTH1qzxkBmph7zK7cVSoR5xvRV_DIGfMsI8fl4ee7XIheMdHA*1695852330243*47e9ef9fb2ed30abc8e30fce62b4a2952ab15a1f40b79e98d80367316ba35ca1*161OhrWK-HSCqpqeYiK5Y40w4IySGPyc7DCHH62ixSk~2";

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
        ws_stream
    }

    #[test(tokio::test)]
    async fn test_chat() {
        dotenv().ok();
        let storage = MemoryStore::new();
        let table = Table::default();
        storage.write_table(table);
        let server_handle = tokio::spawn(server::run(storage));
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let ws_stream = setup_conn().await;
        let (mut write, mut read) = ws_stream.split();

        // Create a sample chat message
        let chat_msg = json!({
            "type": "Chat",
            "room_id": "room1",
            "payload": "Hello, world!",
        })
        .to_string();

        info!("Sending message to server from client");
        // Send the chat message
        write
            .send(Message::Text(chat_msg.clone()))
            .await
            .expect("Failed to send message");

        info!("Message sent");
        // Wait for the message to come back
        if let Some(msg) = read.next().await {
            let msg = msg.expect("Failed to read message");
            match msg {
                Message::Text(text) => {
                    assert_eq!(text, chat_msg);
                }
                _ => panic!("Received unexpected message type"),
            }
        } else {
            panic!("Did not receive a reply");
        }

        server_handle.abort();
    }
}
