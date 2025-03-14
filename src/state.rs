use crate::db::Database;
use crate::models::room::RoomId;
use std::sync::Arc;
use tokio::sync::broadcast;

// Type alias for room events broadcast
pub type RoomEventSender = broadcast::Sender<RoomEvent>;

// Define room events structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "eventType", content = "payload")]
pub enum RoomEvent {
    UserJoined(crate::models::user::User),
    UserLeft(UserLeftPayload),
    VoteSubmitted(UserLeftPayload), // Reusing the simple UUID payload
    VotesRevealed(VotesRevealedPayload),
    VotesReset(VotesResetPayload),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLeftPayload {
    pub user_id: uuid::Uuid,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VotesRevealedPayload {
    pub votes: Vec<VoteWithUser>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteWithUser {
    pub user_id: uuid::Uuid,
    pub value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VotesResetPayload {}

// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    // SQLite database connection
    pub db: Arc<Database>,

    // Broadcasting channels for real-time updates - one per room
    pub room_events: Arc<dashmap::DashMap<RoomId, RoomEventSender>>,
}

impl AppState {
    pub async fn new() -> Result<Self, crate::error::AppError> {
        // Initialize database connection
        let db = Database::new().await?;

        Ok(Self {
            db: Arc::new(db),
            room_events: Arc::new(dashmap::DashMap::new()),
        })
    }

    // Get event sender for a room
    pub fn get_room_event_sender(&self, room_id: &RoomId) -> Option<RoomEventSender> {
        self.room_events.get(room_id).map(|sender| sender.clone())
    }

    // Create event sender for a room if it doesn't exist
    pub fn ensure_room_event_sender(&self, room_id: &RoomId) -> RoomEventSender {
        if let Some(sender) = self.room_events.get(room_id) {
            sender.clone()
        } else {
            let (sender, _) = broadcast::channel(100);
            self.room_events.insert(room_id.clone(), sender.clone());
            sender
        }
    }

    // Remove event sender for a room
    pub fn remove_room_event_sender(&self, room_id: &RoomId) {
        self.room_events.remove(room_id);
    }
}
