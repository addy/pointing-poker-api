use crate::error::AppError;
use crate::models::room::{Room, RoomId, RoomState};
use crate::models::user::{User, UserId};
use crate::models::vote::Vote;
#[allow(unused_imports)]
use sqlx::{Pool, Row, Sqlite, migrate::MigrateDatabase as _, sqlite::SqlitePool};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new() -> Result<Self, AppError> {
        use sqlx::migrate::MigrateDatabase;

        // Database URL for SQLx 0.8
        let db_url = "sqlite:pointing_poker.db";

        // Check if database exists, create it if not
        if !Sqlite::database_exists(db_url).await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to check if database exists: {}", e))
        })? {
            // Database doesn't exist, create it
            Sqlite::create_database(db_url).await.map_err(|e| {
                AppError::DatabaseError(format!("Failed to create database: {}", e))
            })?;

            println!("Database created at {}", db_url);
        }

        // Create connection pool
        let pool = SqlitePool::connect(db_url).await.map_err(|e| {
            AppError::DatabaseError(format!("Failed to connect to database: {}", e))
        })?;

        // Create schema
        Self::create_schema(&pool).await?;

        Ok(Self { pool })
    }

    async fn create_schema(pool: &Pool<Sqlite>) -> Result<(), AppError> {
        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Create rooms table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rooms (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                state TEXT NOT NULL,
                owner_id TEXT
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                is_observer INTEGER NOT NULL,
                room_id TEXT NOT NULL,
                FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Create votes table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS votes (
                user_id TEXT PRIMARY KEY,
                room_id TEXT NOT NULL,
                vote TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
                FOREIGN KEY (room_id) REFERENCES rooms (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    // Room operations
    pub async fn create_room(&self, room: &Room) -> Result<(), AppError> {
        let room_id = room.id.to_string();
        let state_str = match room.state {
            RoomState::Voting => "voting",
            RoomState::Revealed => "revealed",
        };
        let owner_id = room.owner_id.as_ref().map(|id| id.to_string());

        sqlx::query(
            r#"
            INSERT INTO rooms (id, name, state, owner_id)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(room_id)
        .bind(&room.name)
        .bind(state_str)
        .bind(owner_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Add initial users if any
        for user in room.users.values() {
            self.add_user(user, &room.id).await?;
        }

        Ok(())
    }

    pub async fn get_room(&self, room_id: &RoomId) -> Result<Option<Room>, AppError> {
        let room_id_str = room_id.to_string();

        // Get room data
        let room_data = sqlx::query("SELECT name, state, owner_id FROM rooms WHERE id = ?")
            .bind(&room_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let Some(row) = room_data else {
            return Ok(None);
        };

        // Extract room data - in SQLx 0.8, get() method remains the same
        let name: String = row.get("name");
        let state_str: String = row.get("state");
        let owner_id_str: Option<String> = row.get("owner_id");

        // Get users for this room
        let users = self.get_users_for_room(room_id).await?;

        // Get votes for this room
        let votes = self.get_votes_for_room(room_id).await?;

        // Convert to Room model
        let state = match state_str.as_str() {
            "voting" => RoomState::Voting,
            "revealed" => RoomState::Revealed,
            _ => return Err(AppError::DatabaseError("Invalid room state".to_string())),
        };

        let owner_id = if let Some(id_str) = owner_id_str {
            Some(UserId(Uuid::from_str(&id_str).map_err(|e| {
                AppError::DatabaseError(format!("Invalid UUID: {}", e))
            })?))
        } else {
            None
        };

        Ok(Some(Room {
            id: room_id.clone(),
            name,
            state,
            users,
            votes,
            owner_id,
        }))
    }

    pub async fn get_users_for_room(
        &self,
        room_id: &RoomId,
    ) -> Result<HashMap<UserId, User>, AppError> {
        let room_id_str = room_id.to_string();

        // Get users
        let rows = sqlx::query("SELECT id, name, is_observer FROM users WHERE room_id = ?")
            .bind(&room_id_str)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Convert to HashMap<UserId, User>
        let mut user_map = HashMap::new();
        for row in rows {
            let id_str: String = row.get("id");
            let name: String = row.get("name");
            let is_observer: i64 = row.get("is_observer");

            let user_id = UserId(
                Uuid::from_str(&id_str)
                    .map_err(|e| AppError::DatabaseError(format!("Invalid UUID: {}", e)))?,
            );

            let user = User {
                id: user_id.clone(),
                name,
                is_observer: is_observer != 0,
            };

            user_map.insert(user_id, user);
        }

        Ok(user_map)
    }

    pub async fn get_votes_for_room(
        &self,
        room_id: &RoomId,
    ) -> Result<HashMap<UserId, Vote>, AppError> {
        let room_id_str = room_id.to_string();

        // Get votes
        let rows = sqlx::query("SELECT user_id, vote FROM votes WHERE room_id = ?")
            .bind(&room_id_str)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Convert to HashMap<UserId, Vote>
        let mut vote_map = HashMap::new();
        for row in rows {
            let user_id_str: String = row.get("user_id");
            let vote_str: String = row.get("vote");

            let user_id = UserId(
                Uuid::from_str(&user_id_str)
                    .map_err(|e| AppError::DatabaseError(format!("Invalid UUID: {}", e)))?,
            );

            let vote = Vote::from_string(&vote_str).map_err(AppError::DatabaseError)?;

            vote_map.insert(user_id, vote);
        }

        Ok(vote_map)
    }

    pub async fn update_room_state(
        &self,
        room_id: &RoomId,
        state: &RoomState,
    ) -> Result<(), AppError> {
        let room_id_str = room_id.to_string();
        let state_str = match state {
            RoomState::Voting => "voting",
            RoomState::Revealed => "revealed",
        };

        sqlx::query("UPDATE rooms SET state = ? WHERE id = ?")
            .bind(state_str)
            .bind(&room_id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_room(&self, room_id: &RoomId) -> Result<bool, AppError> {
        let room_id_str = room_id.to_string();

        let result = sqlx::query("DELETE FROM rooms WHERE id = ?")
            .bind(&room_id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // User operations
    pub async fn add_user(&self, user: &User, room_id: &RoomId) -> Result<(), AppError> {
        let user_id = user.id.to_string();
        let room_id_str = room_id.to_string();
        let is_observer = user.is_observer as i64;

        sqlx::query(
            r#"
            INSERT INTO users (id, name, is_observer, room_id)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&user_id)
        .bind(&user.name)
        .bind(is_observer)
        .bind(&room_id_str)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn remove_user(&self, user_id: &UserId) -> Result<Option<(User, RoomId)>, AppError> {
        let user_id_str = user_id.to_string();

        // First get user data
        let row = sqlx::query("SELECT name, is_observer, room_id FROM users WHERE id = ?")
            .bind(&user_id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let name: String = row.get("name");
        let is_observer: i64 = row.get("is_observer");
        let room_id_str: String = row.get("room_id");

        // Now delete the user
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(&user_id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Remove their vote too
        self.remove_vote(user_id).await?;

        // Create User and RoomId objects
        let room_id = RoomId::from_string(&room_id_str)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let user = User {
            id: user_id.clone(),
            name,
            is_observer: is_observer != 0,
        };

        Ok(Some((user, room_id)))
    }

    pub async fn count_users_in_room(&self, room_id: &RoomId) -> Result<i64, AppError> {
        let room_id_str = room_id.to_string();

        let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE room_id = ?")
            .bind(&room_id_str)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let count: i64 = row.get("count");

        Ok(count)
    }

    pub async fn update_room_owner(
        &self,
        room_id: &RoomId,
        owner_id: Option<&UserId>,
    ) -> Result<(), AppError> {
        let room_id_str = room_id.to_string();
        let owner_id_str = owner_id.map(|id| id.to_string());

        sqlx::query("UPDATE rooms SET owner_id = ? WHERE id = ?")
            .bind(owner_id_str)
            .bind(&room_id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    // Vote operations
    pub async fn add_vote(
        &self,
        room_id: &RoomId,
        user_id: &UserId,
        vote: &Vote,
    ) -> Result<(), AppError> {
        // Fetch the room first to use its model functionality
        let _room = self
            .get_room(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Room not found".to_string()))?;

        // Now save the vote to the database
        let room_id_str = room_id.to_string();
        let user_id_str = user_id.to_string();
        let vote_val = vote
            .value()
            .ok_or_else(|| AppError::DatabaseError("Invalid vote value".to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO votes (user_id, room_id, vote)
            VALUES (?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET vote = excluded.vote
            "#,
        )
        .bind(&user_id_str)
        .bind(&room_id_str)
        .bind(&vote_val)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn remove_vote(&self, user_id: &UserId) -> Result<(), AppError> {
        let user_id_str = user_id.to_string();

        sqlx::query("DELETE FROM votes WHERE user_id = ?")
            .bind(&user_id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub async fn reset_votes_for_room(&self, room_id: &RoomId) -> Result<(), AppError> {
        let room_id_str = room_id.to_string();

        sqlx::query("DELETE FROM votes WHERE room_id = ?")
            .bind(&room_id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Also reset room state to voting
        self.update_room_state(room_id, &RoomState::Voting).await?;

        Ok(())
    }

    // Method to reveal votes in a room (changes room state to revealed)
    pub async fn reveal_votes(&self, room_id: &RoomId, user_id: &UserId) -> Result<(), AppError> {
        // Get the room first to check if the user is the owner
        let room = self
            .get_room(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Room not found".to_string()))?;

        // Check if the user is the room owner
        if room.owner_id.as_ref() != Some(user_id) {
            return Err(AppError::Forbidden(
                "Only the room owner can reveal votes".to_string(),
            ));
        }

        // Update the room state to revealed
        self.update_room_state(room_id, &RoomState::Revealed)
            .await?;

        Ok(())
    }

    // Method to reset votes in a room
    pub async fn reset_votes(&self, room_id: &RoomId, user_id: &UserId) -> Result<(), AppError> {
        // Get the room first to check if the user is the owner
        let room = self
            .get_room(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Room not found".to_string()))?;

        // Check if the user is the room owner
        if room.owner_id.as_ref() != Some(user_id) {
            return Err(AppError::Forbidden(
                "Only the room owner can reset votes".to_string(),
            ));
        }

        // Reset votes and state
        self.reset_votes_for_room(room_id).await?;

        Ok(())
    }
}
