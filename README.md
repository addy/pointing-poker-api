# Pointing Poker API

A modern Rust HTTP API for a pointing poker application built with Axum. This API supports creating rooms, joining sessions, voting with a Fibonacci scale, and provides real-time updates via WebSockets.

## Features

- REST API for room and vote management
- WebSocket support for real-time updates
- SQLite persistence
- Fibonacci scale for pointing (0, 1, 2, 3, 5, 8, 13, 21)
- Observer mode for non-voting participants

## Project Structure

```
pointing-poker-api/
├── Cargo.toml                   # Project dependencies and metadata
├── src/
│   ├── main.rs                  # Application entry point
│   ├── db.rs                    # Database interactions
│   ├── error.rs                 # Error handling
│   ├── models.rs                # Models module declaration
│   ├── routes.rs                # Routes module declaration with router creation
│   ├── state.rs                 # Application state
│   │
│   ├── models/                  # Models implementation
│   │   ├── room.rs              # Room model
│   │   ├── user.rs              # User model
│   │   └── vote.rs              # Vote model
│   │
│   └── routes/                  # Route handlers implementation
│       ├── room.rs              # Room management endpoints
│       ├── vote.rs              # Voting endpoints
│       └── ws.rs                # WebSocket handling
```

## API Endpoints

### Room Management

- `POST /rooms` - Create a new room
- `GET /rooms/:room_id` - Get room details
- `POST /rooms/:room_id/join` - Join a room
- `POST /rooms/:room_id/leave/:user_id` - Leave a room

### Voting

- `POST /rooms/:room_id/vote` - Submit a vote
- `POST /rooms/:room_id/reveal` - Reveal all votes
- `POST /rooms/:room_id/reset` - Reset votes for a new round

### WebSocket

- `GET /ws/rooms/:room_id/users/:user_id` - WebSocket connection for real-time updates

## Real-time Events

The WebSocket connection provides real-time updates with the following events:

- `UserJoined` - When a new user joins the room
- `UserLeft` - When a user leaves the room
- `VoteSubmitted` - When a vote is submitted (without revealing the value)
- `VotesRevealed` - When the room owner reveals all votes
- `VotesReset` - When votes are reset for a new round
- `RoomUpdated` - General room state changes

## Getting Started

### Prerequisites

- Rust and Cargo (latest stable version)
- SQLite (included as a dependency)

### Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/pointing-poker-api.git
   cd pointing-poker-api
   ```

2. Build the project:

   ```bash
   cargo build --release
   ```

3. Run the server:
   ```bash
   cargo run --release
   ```

The server will start on `http://localhost:3000`.

## Database

The application uses SQLite for persistence through the Rusqlite library with async support via tokio-rusqlite. The database file `pointing_poker.db` will be created automatically in the root directory when the application starts. The schema includes tables for:

- Rooms
- Users
- Votes

## Example Usage

You can use the provided JavaScript client example to interact with the API:

```javascript
const client = new PointingPokerClient();

// Create a room
const room = await client.createRoom("Sprint Planning", "Scrum Master");

// Join a room
await client.joinRoom(room.id, "Developer", false);

// Submit a vote
await client.submitVote("5");

// Reveal votes (only room owner can do this)
await client.revealVotes();

// Reset votes for next round
await client.resetVotes();
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
