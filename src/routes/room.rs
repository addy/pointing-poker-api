use crate::error::AppError;
use crate::models::room::{CreateRoomRequest, Room, RoomId};
use crate::models::user::{User, UserId};
use crate::state::{AppState, RoomEvent};
use axum::{
    Json,
    extract::{Path, State},
};
use std::sync::Arc;

// Create a new room
pub async fn create_room(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateRoomRequest>,
) -> Result<Json<Room>, AppError> {
    // Create user if creator name was provided
    let owner = request.creator_name.map(|name| User::new(name, false));

    // Create a new room
    let room = Room::new(request.name.clone(), owner);
    let room_id = room.id.clone();

    // Store room in database
    state.db.create_room(&room).await?;

    // Create event channel for this room
    let event_sender = state.ensure_room_event_sender(&room_id);

    // Notify that a room was created
    let _ = event_sender.send(RoomEvent::RoomUpdated {
        room_id: room_id.to_string(),
    });

    // Return the newly created room
    Ok(Json(room))
}

// Get room details
pub async fn get_room(
    State(state): State<Arc<AppState>>,
    Path(room_id_str): Path<String>,
) -> Result<Json<Room>, AppError> {
    // Parse room ID
    let room_id = RoomId::from_string(&room_id_str)
        .map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?;

    // Get room from database
    let room = state
        .db
        .get_room(&room_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Room not found".to_string()))?;

    Ok(Json(room))
}

// Join a room
pub async fn join_room(
    State(state): State<Arc<AppState>>,
    Path(room_id_str): Path<String>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<User>, AppError> {
    // Parse room ID
    let room_id = RoomId::from_string(&room_id_str)
        .map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?;

    // Check if room exists
    let room_exists = state.db.get_room(&room_id).await?.is_some();

    if !room_exists {
        return Err(AppError::NotFound("Room not found".to_string()));
    }

    // Create user
    let is_observer = request.is_observer.unwrap_or(false);
    let user = User::new(request.name, is_observer);
    let user_id = user.id.clone();
    let user_name = user.name.clone();

    // Add user to room in database
    state.db.add_user(&user, &room_id).await?;

    // Notify about new user
    let event_sender = state.ensure_room_event_sender(&room_id);
    let _ = event_sender.send(RoomEvent::UserJoined {
        room_id: room_id.to_string(),
        user_id: user_id.to_string(),
        user_name,
    });

    Ok(Json(user))
}

// Leave a room
pub async fn leave_room(
    State(state): State<Arc<AppState>>,
    Path((room_id_str, user_id_str)): Path<(String, String)>,
) -> Result<Json<User>, AppError> {
    // Parse IDs
    let room_id = RoomId::from_string(&room_id_str)
        .map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?;

    let user_id = UserId::from_string(&user_id_str)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    // Remove user from database and get user data
    let (user, _) = state
        .db
        .remove_user(&user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found in room".to_string()))?;

    // Notify about user leaving
    if let Some(tx) = state.get_room_event_sender(&room_id) {
        let _ = tx.send(RoomEvent::UserLeft {
            room_id: room_id.to_string(),
            user_id: user_id.to_string(),
        });
    }

    // Check if this was the room owner
    let room = state.db.get_room(&room_id).await?;

    if let Some(room) = room {
        // If the owner is leaving, assign a new owner if there are other users
        if room.owner_id.as_ref() == Some(&user_id) {
            // Count remaining users
            let user_count = state.db.count_users_in_room(&room_id).await?;

            if user_count > 0 {
                // Get first remaining user
                let users = state.db.get_users_for_room(&room_id).await?;
                if let Some((first_user_id, _)) = users.iter().next() {
                    // Assign as new owner
                    state
                        .db
                        .update_room_owner(&room_id, Some(first_user_id))
                        .await?;
                }
            } else {
                // If room is empty, remove it and its event sender
                state.db.delete_room(&room_id).await?;
                state.remove_room_event_sender(&room_id);
            }
        }
    }

    Ok(Json(user))
}

// Import CreateUserRequest
use crate::models::user::CreateUserRequest;
