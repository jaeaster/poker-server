use crate::storage::MemoryStore;
use axum::{routing::get, Router};
use chashmap::CHashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

mod handlers;

pub type RoomId = String;
pub type PlayerId = String;
type PlayerChannel = mpsc::UnboundedSender<String>;

type Room = Vec<PlayerId>;
type Rooms = CHashMap<RoomId, Room>;
type Sockets = CHashMap<PlayerId, PlayerChannel>;

// Ensure all state is thread-safe since it will be shared
pub struct GlobalState {
    pub sockets: Sockets,
    pub rooms: Rooms,
    pub storage: MemoryStore,
}

// Arc allows references to be shared across threads/tasks
pub struct Context {
    state: Arc<GlobalState>,
    player_id: Arc<PlayerId>,
}

impl Clone for Context {
    // Increment reference counters when cloning
    fn clone(&self) -> Self {
        Context {
            state: self.state.clone(),
            player_id: self.player_id.clone(),
        }
    }
}

pub async fn run() {
    let storage = MemoryStore::new();
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
