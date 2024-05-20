use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::SET_COOKIE, HeaderMap},
    response::{AppendHeaders, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_messages::Messages;
use jsonwebtoken::{encode, EncodingKey, Header};
use time::Duration;
use tokio::sync::RwLock;
use tower_sessions::Session;

use crate::{
    handler::set_tzone_in_session,
    model::{LoginUserSchema, RegisterUserSchema, TokenClaims},
    service::{check_email_password, create_user, get_all_todos},
    AppState,
};

use super::{
    get_messages, set_flag_in_session, Error404Template, Error500Template, HomeTemplate,
    HtmlTemplate, LoginTemplate, RegisterTemplate, FROM_PROTECTED_KEY,
};

/* --------------------------------------- */
/* ------------ Auth Handlers ------------ */
/* --------------------------------------- */

/// Handler to serve the Home Page template.
pub async fn home_handler(session: Session) -> impl IntoResponse {
    let from_protected: bool = session
        .get(FROM_PROTECTED_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    // println!("FP - home page: {}", from_protected.0);

    HtmlTemplate(HomeTemplate {
        title: "Home".to_string(),
        from_protected,
        ..Default::default()
    })
}

/// Handler to serve the Register Page template.
pub async fn register_page_handler(session: Session, messages: Messages) -> impl IntoResponse {
    let from_protected: bool = session
        .get(FROM_PROTECTED_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    let (messages_status, messages) = get_messages(messages);

    HtmlTemplate(RegisterTemplate {
        title: "Register".to_string(),
        messages_status,
        messages,
        from_protected,
        ..Default::default()
    })
}

/// Handle the `POST` request of the user register form.
pub async fn register_user_handler(
    messages: Messages,
    State(state): State<Arc<RwLock<AppState>>>,
    Form(form_data): Form<RegisterUserSchema>,
) -> impl IntoResponse {
    // println!("{:?}", form_data);

    let result = create_user(
        form_data.email,
        form_data.password,
        form_data.username,
        &state.read().await.pool,
    )
    .await;

    if let Err(err) = result {
        let err = format!("Something went wrong: {}", err);
        messages.error(err);

        return Redirect::to("/register");
    }

    // println!("{:?}", result.unwrap());

    messages.success("You have successfully registered!!");

    Redirect::to("/login")
}

/// Handler to serve the Login Page template.
pub async fn login_page_handler(session: Session, messages: Messages) -> impl IntoResponse {
    let from_protected: bool = session
        .get(FROM_PROTECTED_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    let (messages_status, messages) = get_messages(messages);

    HtmlTemplate(LoginTemplate {
        title: "Login".to_string(),
        messages_status,
        messages,
        from_protected,
        ..Default::default()
    })
}

/// Handle the `POST` request of the user login form.
pub async fn login_user_handler(
    headers: HeaderMap,
    session: Session,
    messages: Messages,
    State(state): State<Arc<RwLock<AppState>>>,
    Form(form_data): Form<LoginUserSchema>,
) -> Response {
    let tzone = headers["x-timezone"].to_str().unwrap().to_string();
    set_tzone_in_session(&session, tzone).await;

    let result = check_email_password(
        form_data.email,
        form_data.password,
        &state.read().await.pool,
    )
    .await;

    if let Err(err) = result {
        let err = format!("Something went wrong: {}", err);
        messages.error(err);

        return Redirect::to("/login").into_response();
    }

    let user_id = result.unwrap().id;

    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
    let claims = TokenClaims {
        sub: user_id.clone(),
        exp,
        iat,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&state.read().await.config.jwt_secret.as_ref()),
    )
    .unwrap();

    let cookie = Cookie::build(("token", token.to_owned()))
        .path("/")
        .max_age(Duration::hours(1))
        .same_site(SameSite::Lax)
        .http_only(true);

    let headers = AppendHeaders([(SET_COOKIE, cookie.to_string())]);

    let lock = state.read().await;
    let result = get_all_todos(user_id, &lock.pool).await;
    if let Err(e) = result {
        return HtmlTemplate(Error500Template {
            title: "Error 500".to_string(),
            reason: e.to_string(),
            link: "/".to_string(),
            is_error: true,
            ..Default::default()
        })
        .into_response();
    }
    drop(lock);

    let mut lock = state.write().await;

    lock.todos = result.unwrap();
    drop(lock);

    messages.success("You have successfully logged in!!");

    (headers, Redirect::to("/todo/list")).into_response()
}

/// User Logout Handler.
pub async fn logout_handler(session: Session, messages: Messages) -> impl IntoResponse {
    set_flag_in_session(&session, false).await;

    let cookie = Cookie::build(("token", ""))
        .path("/")
        .max_age(Duration::hours(-1))
        .same_site(SameSite::Lax)
        .http_only(true);

    let headers = AppendHeaders([(SET_COOKIE, cookie.to_string())]);

    messages.success("You have successfully logged out!!");

    (headers, Redirect::to("/login"))
}

/* --------------------------------------- */
/* ----------- Eror 404 Handler ---------- */
/* --------------------------------------- */

/// Global Error 404 Handler (to handle unknown paths).
pub async fn handler_404(session: Session) -> impl IntoResponse {
    let from_protected: bool = session
        .get(FROM_PROTECTED_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    let link = if from_protected {
        "/todo/list".to_string()
    } else {
        "/".to_string()
    };

    HtmlTemplate(Error404Template {
        title: "Error 404".to_string(),
        reason: "Nothing to see here".to_string(),
        link,
        // from_protected,
        is_error: true,
        ..Default::default()
    })
}

/* INITIALIZE FIELDS WITH DEFAULT VALUES OF A STRUCTURE:
https://stackoverflow.com/questions/19650265/is-there-a-faster-shorter-way-to-initialize-variables-in-a-rust-struct
https://moneygrowsontrees.medium.com/how-default-values-and-optional-parameters-work-in-rust-d0a3972621bc
*/

/* REFERENCES:
https://github.com/tokio-rs/axum/discussions/351
https://stackoverflow.com/questions/77579968/cookie-passed-when-expected
https://docs.rs/axum/latest/axum/response/struct.AppendHeaders.html

https://github.com/maxcountryman/tower-sessions

https://spacedimp.com/blog/using-rust-axum-postgresql-and-tokio-to-build-a-blog/

How do I get last commit date from git repository?
https://stackoverflow.com/questions/25563455/how-do-i-get-last-commit-date-from-git-repository
*/
