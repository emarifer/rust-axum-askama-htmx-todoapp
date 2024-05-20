use std::result::Result::Ok;

use anyhow::{anyhow, bail, Result};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use sqlx::{query, query_as, query_scalar, SqlitePool};
use uuid::Uuid;

use crate::model::{Todo, User};

pub async fn create_user(
    email: String,
    password: String,
    username: String,
    pool: &SqlitePool,
) -> Result<User> {
    // Check if the email is already in use
    let user_exists: Option<bool> =
        query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
            .bind(email.to_ascii_lowercase())
            .fetch_one(pool)
            .await?;

    if let Some(exists) = user_exists {
        if exists {
            // Err(anyhow!("the email is already in use."))?;
            bail!("the email is already in use.");
            // Equivalent to:
            // return Err(anyhow!("the email is already in use."));
            // ↓↓ SEE NOTE-01 BELOW ↓↓
        }
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("failed to hash password: {}", e))
        .map(|hash| hash.to_string())?;

    let uuid = Uuid::new_v4().to_string();

    let user = query_as!(
        User,
        "INSERT INTO users (id,email,password,username) VALUES ($1, $2, $3, $4) RETURNING *",
        uuid,
        email,
        hashed_password,
        username
    )
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow!("database error: {}", e))?;

    Ok(user)
}

pub async fn check_email_password(
    email: String,
    password: String,
    pool: &SqlitePool,
) -> Result<User> {
    let email = email.to_ascii_lowercase();
    let user = query_as!(User, "SELECT * FROM users WHERE email = $1", email)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("database error: {}.", e))?
        .ok_or_else(|| anyhow!("invalid email or password."))?;

    let is_valid = match PasswordHash::new(&user.password) {
        Ok(parsed_hash) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_or(false, |_| true),
        Err(_err) => false,
    };

    if !is_valid {
        bail!("invalid email or password.");
    }

    Ok(user)
}

pub async fn get_user_by_id(user_id: &str, pool: &SqlitePool) -> Result<Option<User>, String> {
    query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("error fetching user from database: {}", e))
}

pub async fn add_todo(
    created_by: String,
    title: String,
    description: String,
    pool: &SqlitePool,
) -> Result<Todo> {
    let todo = query_as!(
        Todo,
        "INSERT INTO todos (created_by,title,description) VALUES($1, $2, $3) RETURNING *",
        created_by,
        title,
        description,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow!("database error: {}", e))?;

    Ok(todo)
}

pub async fn get_all_todos(created_by: String, pool: &SqlitePool) -> Result<Vec<Todo>> {
    let todos = query_as!(
        Todo,
        "SELECT * FROM todos WHERE created_by = ? ORDER BY created_at DESC",
        created_by
    )
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow!("database error: {}", e))?;

    Ok(todos)
}

pub async fn get_todo_by_id(todo_id: i64, pool: &SqlitePool) -> Result<Todo> {
    let todo = query_as!(Todo, "SELECT * FROM todos WHERE id = $1", todo_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow!("database error: {}.", e))?
        .ok_or_else(|| anyhow!("todo does not exist in the database."))?;

    Ok(todo)
}

pub async fn remove_todo(todo_id: i64, pool: &SqlitePool) -> Result<()> {
    let rows_affected = query!("DELETE FROM todos WHERE id = $1", todo_id)
        .execute(pool)
        .await
        .unwrap()
        .rows_affected();

    if rows_affected == 0 {
        bail!(format!("Todo with ID: {} not found", todo_id));
    }

    Ok(())
}

pub async fn update_todo(
    title: String,
    description: String,
    status: bool,
    todo_id: i64,
    pool: &SqlitePool,
) -> Result<()> {
    let rows_affected = query!(
        "UPDATE todos SET title = $1, description = $2, status = $3 WHERE id = $4",
        title,
        description,
        status,
        todo_id
    )
    .execute(pool)
    .await
    .unwrap()
    .rows_affected();

    if rows_affected == 0 {
        bail!(format!("Todo with ID: {} not found", todo_id));
    }

    Ok(())
}

/* NOTE-01:
https://antoinerr.github.io/blog-website/2023/01/28/rust-anyhow.html#returning-early-with-an-error
*/

/* HANDLE PASSWORD HASH GENERATION:
https://gist.github.com/DefectingCat/749e1d291133198a995f252a8d610628
*/

/* CAUSE A FAILURE IN THE GENERATION OF THE PASSWORD HASH TO CHECK FOR ERRORS:
    let hp: Result<String, crate::service::password_hash::Error> = Err(
        password_hash::Error::B64Encoding(password_hash::errors::B64Error::InvalidEncoding),
    );
    let hp: Result<String, crate::service::password_hash::Error> =
        Err(password_hash::Error::Version);
    let hashed_password = hp
        .map_err(|e| anyhow!("failed to hash password: {}", e))
        .map(|_| "hashed_password".to_owned())?;
    https://docs.rs/password-hash/0.5.0/password_hash/errors/enum.Error.html
*/

/* MISCELLANEOUS:
https://doc.rust-lang.org/std/result/enum.Result.html#method.map_err
*/
