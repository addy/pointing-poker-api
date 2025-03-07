pub mod room;
pub mod vote;
pub mod ws;

use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

/// Creates the application router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Room routes
        .route("/rooms", post(room::create_room))
        .route("/rooms/{room_id}", get(room::get_room))
        .route("/rooms/{room_id}/join", post(room::join_room))
        .route("/rooms/{room_id}/leave/{user_id}", post(room::leave_room))
        // Voting routes
        .route("/rooms/{room_id}/vote", post(vote::submit_vote))
        .route("/rooms/{room_id}/reveal", post(vote::reveal_votes))
        .route("/rooms/{room_id}/reset", post(vote::reset_votes))
        // WebSocket route
        .route("/ws/rooms/{room_id}/users/{user_id}", get(ws::ws_handler))
        // Apply state to all routes
        .with_state(state)
}

/// Simple health check endpoint
async fn health_check() -> &'static str {
    "OK"
}
