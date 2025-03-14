use crate::error::AppError;
use crate::models::room::RoomId;
use crate::models::user::UserId;
use crate::models::vote::{Vote, VoteRequest};
use crate::state::{AppState, RoomEvent};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
pub struct VoteResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Deserialize)]
pub struct SubmitVoteRequest {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub vote: VoteRequest,
}

// Submit a vote
pub async fn submit_vote(
    State(state): State<Arc<AppState>>,
    Path(room_id_str): Path<String>,
    Json(payload): Json<SubmitVoteRequest>,
) -> Result<Json<VoteResponse>, AppError> {
    // Parse room ID
    let room_id = RoomId::from_string(&room_id_str)
        .map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?;

    // Parse user ID
    let user_id = UserId::from_string(&payload.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    // Parse vote
    let vote = Vote::from_string(&payload.vote.value).map_err(AppError::BadRequest)?;

    // Add vote to database (validation happens in the model through db.add_vote)
    state.db.add_vote(&room_id, &user_id, &vote).await?;

    // Notify about vote submission
    if let Some(tx) = state.get_room_event_sender(&room_id) {
        let _ = tx.send(RoomEvent::VoteSubmitted(crate::state::UserLeftPayload {
            user_id: user_id.0,
        }));
    }

    Ok(Json(VoteResponse {
        success: true,
        message: "Vote submitted successfully".to_string(),
    }))
}

// Reveal votes
pub async fn reveal_votes(
    State(state): State<Arc<AppState>>,
    Path(room_id_str): Path<String>,
    Json(payload): Json<AdminActionRequest>,
) -> Result<Json<VoteResponse>, AppError> {
    // Parse room ID
    let room_id = RoomId::from_string(&room_id_str)
        .map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?;

    // Parse user ID
    let user_id = UserId::from_string(&payload.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    // Reveal votes using domain model logic in database layer
    state.db.reveal_votes(&room_id, &user_id).await?;

    // Notify about votes being revealed
    if let Some(tx) = state.get_room_event_sender(&room_id) {
        // Get room with votes and users
        let room = state.db.get_room(&room_id).await?
            .ok_or_else(|| AppError::NotFound("Room not found".to_string()))?;
        
        // Create vote payloads from room data
        let mut vote_payloads = Vec::new();
        for (user_id, vote) in room.votes.iter() {
            if room.users.contains_key(user_id) {
                vote_payloads.push(crate::state::VoteWithUser {
                    user_id: user_id.0,
                    value: vote.value().unwrap_or_else(|| "hidden".to_string()),
                });
            }
        }
            
        let _ = tx.send(RoomEvent::VotesRevealed(crate::state::VotesRevealedPayload {
            votes: vote_payloads,
        }));
    }

    Ok(Json(VoteResponse {
        success: true,
        message: "Votes revealed successfully".to_string(),
    }))
}

// Reset votes
pub async fn reset_votes(
    State(state): State<Arc<AppState>>,
    Path(room_id_str): Path<String>,
    Json(payload): Json<AdminActionRequest>,
) -> Result<Json<VoteResponse>, AppError> {
    // Parse room ID
    let room_id = RoomId::from_string(&room_id_str)
        .map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?;

    // Parse user ID
    let user_id = UserId::from_string(&payload.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?;

    // Reset votes using domain model logic in database layer
    state.db.reset_votes(&room_id, &user_id).await?;

    // Notify about votes being reset
    if let Some(tx) = state.get_room_event_sender(&room_id) {
        let _ = tx.send(RoomEvent::VotesReset(crate::state::VotesResetPayload {}));
    }

    Ok(Json(VoteResponse {
        success: true,
        message: "Votes reset successfully".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct AdminActionRequest {
    #[serde(rename = "userId")]
    pub user_id: String,
}
