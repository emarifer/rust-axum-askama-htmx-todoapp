mod config;
mod db;
mod handler;
mod model;
mod route;
mod serialization;
mod service;

use std::sync::Arc;

use anyhow::Result;
use dotenv::dotenv;
use model::Todo;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::config::Config;

/// This structure represents the state of the application,
/// holding a database connection pool and app config data
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Config,
    pub todos: Vec<Todo>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from the `.env` file
    dotenv().ok();

    // Retrieve the value of the `DATABASE_URL` from .env file
    let config = Config::init();

    // Connect to `Sqlite` database
    let pool = db::connect(&config.database_url).await?;

    let todos: Vec<Todo> = vec![];

    // Set up the application state with the provided
    // database connection pool and app config data
    let app_state = Arc::new(RwLock::new(AppState {
        pool,
        config,
        todos,
    }));

    // Start the http server
    route::serve(app_state).await?;

    Ok(())
}

/* HOT RELOADING COMMAND:
cargo watch -x run -w src -w assets -w templates
*/

/* REFERENCES:
https://codevoweb.com/jwt-authentication-in-rust-using-axum-framework/
https://codevoweb.com/rust-api-user-registration-and-email-verification/
https://github.com/wpcodevo/rust-axum-jwt-auth
https://github.com/wpcodevo/rust-user-signup-forgot-password-email
https://github.com/emarifer/axum-auth-crud-yew-app/blob/main/src/middleware.rs
https://docs.rs/axum/latest/axum/middleware/fn.from_fn.html
https://github.com/maxcountryman/axum-messages
https://github.com/maxcountryman/axum-login
https://github.com/maxcountryman/tower-sessions
https://docs.rs/tower-sessions/latest/tower_sessions/

https://belkadan.com/blog/2023/10/Type-Erasure-in-Rust/
*/
