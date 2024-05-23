use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::serialization::{deserialize_checkbox, false_fn};

/// Struct to read/write user data in the pool.
#[derive(Debug, Default, Clone, Deserialize, FromRow, Serialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password: String,
    pub username: String,
}

/// Struct for holding data from the user register form.
#[derive(Debug, Deserialize)]
pub struct RegisterUserSchema {
    pub email: String,
    pub password: String,
    pub username: String,
}

/// Struct for holding data from the user login form.
#[derive(Debug, Deserialize)]
pub struct LoginUserSchema {
    pub email: String,
    pub password: String,
}

/// Struct for holding data from the JWT.
#[derive(Debug, Deserialize, Serialize)]
pub struct TokenClaims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
}

/// Structure that represents an row from the `todos` table.
#[derive(Clone, Debug, Default, Deserialize, FromRow, Serialize)]
pub struct Todo {
    pub id: i64,
    pub created_by: String,
    pub title: String,
    pub description: String,
    pub status: bool,
    pub created_at: NaiveDateTime,
}

/// Struct for holding data from the todo create form.
#[derive(Debug, Deserialize)]
pub struct TodoSchema {
    pub title: String,
    pub description: String,
}

/// Struct for holding data from the todo edit form.
#[derive(Debug, Deserialize)]
pub struct TodoEditSchema {
    pub title: String,
    pub description: String,
    #[serde(default = "false_fn")]
    #[serde(deserialize_with = "deserialize_checkbox")]
    pub status: bool,
}
