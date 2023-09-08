use crate::*;
use axum::{
    extract::{ws::WebSocketUpgrade, ConnectInfo, State},
    response::IntoResponse,
    routing::get,
    Router, TypedHeader,
};
use handle_socket::handle_socket;
use std::{collections::HashMap, net::SocketAddr};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

pub mod context;
pub mod cookie;
pub mod handle_socket;
pub mod messages;

pub use context::*;
pub use cookie::*;
pub use messages::*;

pub async fn run() {
    let table = Table::default();
    let room = RoomHandle::new(table);
    let rooms = HashMap::from([(room.id.clone(), room)]);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(rooms.clone())
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
    State(rooms): State<HashMap<RoomId, RoomHandle>>,
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
        rooms,
        session,
        connection_info: ConnectionInfo {
            user_agent,
            ip: addr.to_string(),
        },
    };

    ws.on_upgrade(move |socket| handle_socket(socket, ctx))
}
