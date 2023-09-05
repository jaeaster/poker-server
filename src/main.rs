use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod messages;
mod server;
mod storage;

// Main Entry Point
#[tokio::main]
async fn main() {
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
    use crate::server;
    use futures::{sink::SinkExt, stream::StreamExt};
    use serde_json::json;
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
    use tracing::info;

    #[tokio::test]
    async fn test_chat() {
        // Initialize the tracing subscriber
        let _ = tracing_subscriber::fmt::try_init();

        let server_handle = tokio::spawn(server::run());

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let url = "ws://localhost:8080/ws";
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
        let (mut write, mut read) = ws_stream.split();

        // Create a sample chat message
        let chat_msg = json!({
            "type": "Room",
            "id": "room1",
            "payload": {
                "type": "Chat",
                "content": "Hello, world!"
            }
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
