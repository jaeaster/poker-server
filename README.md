# Poker Server

A WebSocket-based Texas Hold'em poker server built with Rust, Tokio, and Axum.

## Features

- **Real-time WebSocket gameplay** - Low-latency multiplayer poker over WebSockets
- **Actor-based architecture** - Player and Room actors for clean state management
- **Turn timers** - Automatic fold on timeout (configurable, default 30s)
- **Multi-table support** - Room registry supports multiple concurrent tables
- **Lobby system** - Browse available tables and subscribe to rooms
- **Chat** - In-room chat between players

## Architecture

```
┌─────────────┐     WebSocket      ┌─────────────────┐
│   Client    │◄──────────────────►│  Axum Server    │
└─────────────┘                    └────────┬────────┘
                                            │
                                            ▼
                                   ┌─────────────────┐
                                   │ Player Registry │
                                   └────────┬────────┘
                                            │
                    ┌───────────────────────┼───────────────────────┐
                    ▼                       ▼                       ▼
           ┌──────────────┐       ┌──────────────┐        ┌──────────────┐
           │ Player Actor │       │ Player Actor │        │ Player Actor │
           └──────┬───────┘       └──────┬───────┘        └──────┬───────┘
                  │                      │                       │
                  └──────────────────────┼───────────────────────┘
                                         ▼
                                ┌─────────────────┐
                                │  Room Registry  │
                                └────────┬────────┘
                                         ▼
                                ┌─────────────────┐
                                │   Room Actor    │
                                │  ┌───────────┐  │
                                │  │   Table   │  │
                                │  │  ┌─────┐  │  │
                                │  │  │Game │  │  │
                                │  │  └─────┘  │  │
                                │  └───────────┘  │
                                └─────────────────┘
```

## Getting Started

### Prerequisites

- Rust nightly (uses `assert_matches` feature)
- A WebSocket client for testing

### Installation

```bash
# Clone the repository
git clone https://github.com/jaeaster/poker-server.git
cd poker-server

# Copy environment configuration
cp .env.example .env

# Edit .env with your configuration
# IMPORTANT: Change POKER_SESSION_SECRET to a secure random string

# Build
cargo build

# Run
cargo run
```

The server starts on `0.0.0.0:8080` by default.

## Running Tests

```bash
cargo test
```

## WebSocket Protocol

Connect to `ws://localhost:8080/ws` with a session cookie.

### Client Messages

| Message Type | Payload | Description |
|--------------|---------|-------------|
| `getTables` | - | Request list of available tables |
| `subscribe` | `roomId` | Subscribe to room updates |
| `chat` | `roomId`, `message` | Send chat message |
| `sitTable` | `roomId`, `chips` | Sit at table with chips |
| `bet` | `roomId`, `amount` | Place a bet (0 = check, amount = bet/raise) |
| `fold` | `roomId` | Fold hand |
| `sitOutNextHand` | `roomId`, `enabled` | Toggle sit out next hand |
| `sitOutNextBigBlind` | `roomId`, `enabled` | Toggle sit out at next big blind |
| `waitForBigBlind` | `roomId`, `enabled` | Wait for big blind before playing |
| `checkFold` | `roomId`, `enabled` | Auto check/fold when action |
| `callAny` | `roomId`, `enabled` | Auto call any bet |

### Server Messages

| Message Type | Payload | Description |
|--------------|---------|-------------|
| `tableList` | `tables[]` | List of available tables |
| `chat` | `roomId`, `from`, `message` | Chat message broadcast |
| `sitTable` | `roomId`, `player`, `index` | Player sat at table |
| `newGame` | `roomId`, `gameState` | New game started |
| `gameUpdate` | `roomId`, `gameState` | Game state updated |
| `dealHand` | `roomId`, `hand` | Your hole cards |
| `roomError` | `roomId`, `error` | Error message |
| `lobbyError` | `error` | Lobby error message |

## Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `RUST_ENV` | Environment (development/production) | - |
| `POKER_COOKIE_NAME` | Session cookie name | - |
| `POKER_SESSION_SECRET` | Secret for session encryption (min 32 chars) | - |

## Project Structure

```
src/
├── main.rs              # Entry point, env vars, test suite
├── server.rs            # Axum server setup, WebSocket handler
├── server/
│   ├── context.rs       # Request context (session, connection info)
│   ├── cookie.rs        # Iron cookie session management
│   └── handle_socket.rs # WebSocket message handling
├── actors.rs            # Actor system exports
├── actors/
│   ├── player.rs        # Player actor (WebSocket connection)
│   ├── room.rs          # Room actor (game table management)
│   └── registry.rs      # Actor registry (concurrent hashmap)
├── models.rs            # Domain model exports
├── models/
│   ├── player.rs        # Player model
│   ├── table.rs         # Table configuration and seated players
│   └── game.rs          # Game state and poker logic
├── messages.rs          # Message type exports
└── messages/
    ├── client.rs        # Client -> Server messages
    └── server.rs        # Server -> Client messages
```

## Status

This is a functional but incomplete poker server. Working features:

- Lobby browsing and room subscription
- Chat
- Sitting at tables with chips
- Full betting rounds (preflop, flop, turn, river)
- Fold and bet actions
- Turn timers with auto-fold
- Multi-hand games with rotating dealer

Not yet implemented:
- All-in and side pots
- Showdown hand comparison
- Player leaving/disconnection handling
- Blinds structure progression
- Tournament mode

## License

This project is licensed under the GPL-3.0 License - see the [LICENSE](LICENSE) file for details.
