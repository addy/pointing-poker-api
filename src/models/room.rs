use crate::models::user::{User, UserId};
use crate::models::vote::Vote;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RoomId(pub Uuid);

impl RoomId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for RoomId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomState {
    Voting,
    Revealed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub name: String,
    pub state: RoomState,
    pub users: HashMap<UserId, User>,
    pub votes: HashMap<UserId, Vote>,
    pub owner_id: Option<UserId>,
}

impl Room {
    pub fn new(name: String, owner: Option<User>) -> Self {
        let owner_id = owner.as_ref().map(|o| o.id.clone());
        let mut users = HashMap::new();

        if let Some(owner) = owner {
            users.insert(owner.id.clone(), owner);
        }

        Self {
            id: RoomId::new(),
            name,
            state: RoomState::Voting,
            users,
            votes: HashMap::new(),
            owner_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub creator_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRoomRequest {
    pub user: CreateUserRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub is_observer: Option<bool>,
}
