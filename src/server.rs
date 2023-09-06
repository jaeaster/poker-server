use crate::storage::MemoryStore;
use axum::{routing::get, Router};
use chashmap::CHashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use self::cookie::Session;

pub mod context;
pub mod cookie;
pub mod handlers;
pub mod messages;

pub use context::*;
pub use cookie::*;

pub async fn run(storage: MemoryStore) {
    let global_state = GlobalState {
        storage,
        sockets: CHashMap::new(),
        rooms: CHashMap::new(),
    };
    global_state.rooms.insert("room1".to_string(), Vec::new());
    let global_state = Arc::new(global_state);

    let app = Router::new()
        .route("/ws", get(handlers::ws_handler))
        .with_state(global_state.clone())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let addr = "0.0.0.0:8080".parse().expect("Invalid binding address");
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Server Failed");
}
