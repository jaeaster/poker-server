use crate::*;
use axum::{
    extract::{ws::WebSocketUpgrade, ConnectInfo, State},
    response::IntoResponse,
    routing::get,
    Router, TypedHeader,
};
use handle_socket::handle_socket;
use std::net::SocketAddr;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

pub mod context;
pub mod cookie;
pub mod handle_socket;
pub mod messages;

pub use context::*;
pub use cookie::*;
pub use messages::*;

#[derive(Clone)]
pub struct AppState {
    room_registry: RegistryHandle<RoomId, RoomHandle>,
    player_registry: RegistryHandle<PlayerId, PlayerHandle>,
}

pub async fn run() {
    let table = Table::default();
    let player_registry = RegistryHandle::new();
    let room_registry = RegistryHandle::new();
    let room = RoomHandle::new(table, player_registry.clone(), room_registry.clone());
    room_registry.set(room.id.clone(), room).await;
    // Spawns an actor to manage the player registry
    let app_state = AppState {
        room_registry,
        player_registry,
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state)
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
    State(app_state): State<AppState>,
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

    debug!(addr = ?addr, user_agent = user_agent, "New Connection");

    let session = Session::from_cookie(&session_cookie, &crate::COOKIE_SECRET).unwrap();
    let ctx = Context {
        session,
        connection_info: ConnectionInfo {
            user_agent,
            ip: addr.to_string(),
        },
    };

    ws.on_upgrade(move |socket| handle_socket(socket, app_state, ctx))
}
