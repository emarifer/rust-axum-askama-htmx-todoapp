use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, DecodingKey, Validation};
use tokio::sync::RwLock;
use tower_sessions::Session;

use super::{set_flag_in_session, Error401Template, HtmlTemplate};
use crate::{model::TokenClaims, service::get_user_by_id, AppState};

/// Middleware to manage authorization.
pub async fn auth_middleware(
    cookie_jar: CookieJar,
    session: Session,
    State(state): State<Arc<RwLock<AppState>>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    let token_option = cookie_jar
        .get("token")
        .map(|cookie| cookie.value().to_string())
        .or_else(|| {
            req.headers()
                .get(header::AUTHORIZATION)
                .and_then(|auth_header| auth_header.to_str().ok())
                .and_then(|auth_value| {
                    if auth_value.starts_with("Bearer ") {
                        Some(auth_value[7..].to_owned())
                    } else {
                        None
                    }
                })
        });

    // let token = token.ok_or_else(|| "You are not logged in, please provide token")?;

    let token = if let Some(tk) = token_option {
        tk
    } else {
        set_flag_in_session(&session, false).await;

        Err(HtmlTemplate(Error401Template {
            title: "Error 401".to_string(),
            reason: "You are not logged in, please provide token".to_string(),
            is_error: true,
            ..Default::default()
        })
        .into_response())?
    };

    let claims = if let Ok(clm) = decode::<TokenClaims>(
        &token,
        &DecodingKey::from_secret(&state.read().await.config.jwt_secret.as_ref()),
        &Validation::default(),
    ) {
        clm.claims
    } else {
        set_flag_in_session(&session, false).await;

        Err(HtmlTemplate(Error401Template {
            title: "Error 401".to_string(),
            reason: "Invalid token".to_string(),
            is_error: true,
            ..Default::default()
        })
        .into_response())?
    };

    let user_id = &claims.sub;
    let lock = state.read().await;
    let pool = &lock.pool;
    let result = get_user_by_id(user_id, pool).await;
    drop(lock);

    if let Err(e) = result.clone() {
        set_flag_in_session(&session, false).await;

        Err(HtmlTemplate(Error401Template {
            title: "Error 401".to_string(),
            reason: e,
            is_error: true,
            ..Default::default()
        })
        .into_response())?
    };

    let user = if let Some(u) = result.unwrap() {
        u
    } else {
        set_flag_in_session(&session, false).await;

        Err(HtmlTemplate(Error401Template {
            title: "Error 401".to_string(),
            reason: "The user belonging to this token no longer exists".to_string(),
            is_error: true,
            ..Default::default()
        })
        .into_response())?
    };

    set_flag_in_session(&session, true).await;

    req.extensions_mut().insert(user);

    Ok::<Response, _>(next.run(req).await)
}
