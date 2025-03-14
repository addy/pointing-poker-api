use crate::error::AppError;
use crate::models::room::RoomId;
use crate::models::user::UserId;
use crate::state::AppState;
use axum::{
    extract::{Path, State, WebSocketUpgrade, connect_info::ConnectInfo, ws},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
// Removed unused import
use std::net::SocketAddr;
use std::sync::Arc;

// WebSocket handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path((room_id_str, user_id_str)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    // Parse IDs
    let room_id = RoomId::from_string(&room_id_str)
        .map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?;

    let user_id = UserId::from_string(&user_id_str)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    // Verify room exists
    let _room = state
        .db
        .get_room(&room_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Room not found".to_string()))?;

    // Verify user is in this room
    let users = state.db.get_users_for_room(&room_id).await?;
    if !users.contains_key(&user_id) {
        return Err(AppError::BadRequest("User not in room".to_string()));
    }

    // Get or create a broadcast channel for this room
    let tx = state.ensure_room_event_sender(&room_id);
    let mut rx = tx.subscribe();

    // Return the WebSocket connection
    Ok(ws.on_upgrade(move |socket| async move {
        tracing::debug!("WebSocket connected: {}", addr);

        // Split socket into sender and receiver
        let (mut sender, mut receiver) = socket.split();

        // Handle messages from client
        let mut send_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                // The event is already properly typed and structured
                // Serialize the RoomEvent enum directly - it has the correct tag/content structure
                if let Ok(serialized_event) = serde_json::to_string(&msg) {
                    // Send serialized event to client
                    if sender
                        .send(ws::Message::Text(serialized_event.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });
        // Handle messages from client (we mostly ignore them, as clients communicate through REST API)
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(_msg)) = receiver.next().await {
                // Most communication happens through REST API
                // We can process custom WebSocket messages here if needed
            }
        });

        // Wait for either task to finish
        tokio::select! {
            _ = &mut send_task => recv_task.abort(),
            _ = &mut recv_task => send_task.abort(),
        }

        // Log disconnection
        tracing::debug!("WebSocket client disconnected: {}", addr);
    }))
}
