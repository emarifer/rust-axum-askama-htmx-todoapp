use std::sync::Arc;

use anyhow::Result;
use axum::{
    middleware::from_fn_with_state,
    routing::{delete, get, post},
    Router,
};
use axum_messages::MessagesManagerLayer;
use tokio::sync::RwLock;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tower_sessions::{MemoryStore, SessionManagerLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    handler::{
        auth_middleware, handler_404, health_checker_handler, home_handler, login_page_handler,
        login_user_handler, logout_handler, register_page_handler, register_user_handler,
        todo_add_handler, todo_create_handler, todo_delete_handler, todo_edit_handler,
        todo_list_handler, todo_patch_handler,
    },
    AppState,
};

/// This function serves as the entry point for running the Axum web server.
/// It takes a PostgreSQL connection pool (`PgPool`) as input,
/// sets up the application state,
/// creates the API routes using the provided application state,
/// binds the server to a specific port,
/// and starts serving incoming connections.
pub async fn serve(app_state: Arc<RwLock<AppState>>) -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_axum_askama_htmx=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("initializing router…");

    // Create the router using the application state
    let app = create_router(app_state);

    let port = 8082_u16;

    // Bind the server to the specified address and port
    let address = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    info!("🚀 router initialized, now listening on port {}", port);

    // Start serving incoming connections
    axum::serve(address, app.into_make_service()).await?;

    Ok(())
}

/// This function defines the API routes for the application.
/// It takes the application state as input and sets up
/// the routes for handling different HTTP methods and endpoints.
fn create_router(app_state: Arc<RwLock<AppState>>) -> Router {
    // Setup session store for flash messages & globals flags
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    // Get the current directory for serving assets
    let assets_path = std::env::current_dir().unwrap();

    // General router of our application
    Router::new()
        .route("/", get(home_handler))
        .route(
            "/register",
            get(register_page_handler).post(register_user_handler),
        )
        .route("/login", get(login_page_handler).post(login_user_handler))
        .route(
            "/todo/list",
            get(todo_list_handler)
                .route_layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        )
        .route(
            "/logout",
            post(logout_handler)
                .route_layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        )
        .route(
            "/create",
            get(todo_create_handler)
                .post(todo_add_handler)
                .route_layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        )
        .route(
            "/edit",
            get(todo_edit_handler)
                .post(todo_patch_handler)
                .route_layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        )
        .route("/delete", delete(todo_delete_handler))
        .route("/healthchecker", get(health_checker_handler))
        .nest_service(
            "/assets",
            ServeDir::new(format!("{}/assets", assets_path.to_str().unwrap())), // Serve static assets
        )
        .with_state(app_state)
        .fallback(handler_404) // Add a Fallback service for handling unknown paths
        .layer(MessagesManagerLayer)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
}
