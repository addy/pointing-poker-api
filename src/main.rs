mod db;
mod error;
mod models;
mod routes;
mod state;

use crate::error::AppError;
use crate::routes::create_router;
use crate::state::AppState;

use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create application state with database connection
    let app_state = Arc::new(AppState::new().await?);

    info!("Database connection established");

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build application with routes
    let app = create_router(app_state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // Define the address to run the server on
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Starting server on {}", addr);

    // Start the server - updated for Axum 0.8 with Hyper
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::ServerStartupError(e.to_string()))?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| AppError::ServerStartupError(e.to_string()))?;

    Ok(())
}
