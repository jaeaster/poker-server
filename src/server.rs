use self::cookie::Session;
use crate::storage::MemoryStore;
use axum::{
    extract::{ws::WebSocketUpgrade, ConnectInfo, State},
    response::IntoResponse,
    TypedHeader,
};
use axum::{routing::get, Router};
use chashmap::CHashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::debug;

pub mod context;
pub mod cookie;
pub mod handlers;
pub mod messages;

pub use context::*;
pub use cookie::*;
use handlers::handle_socket;

pub async fn run(storage: MemoryStore) {
    let global_state = GlobalState {
        storage,
        sockets: CHashMap::new(),
        rooms: CHashMap::new(),
    };
    global_state.rooms.insert("room1".to_string(), Vec::new());
    let global_state = Arc::new(global_state);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(global_state.clone())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let addr = crate::ADDR.parse().expect("Invalid binding address");
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Server Failed");
}

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

    let session_cookie = if let Some(TypedHeader(cookies)) = cookies {
        cookies.get(&crate::COOKIE_NAME).unwrap().to_string()
    } else {
        String::from("cookie not found")
    };

    debug!("`{}` at {} connected.", user_agent, addr);

    let session = Session::from_cookie(&session_cookie, &crate::COOKIE_SECRET).unwrap();
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
