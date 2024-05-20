mod auth_handler;
mod middleware;
mod todo_handler;

pub use auth_handler::{
    handler_404, home_handler, login_page_handler, login_user_handler, logout_handler,
    register_page_handler, register_user_handler,
};
use chrono::{Local, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
pub use middleware::auth_middleware;
pub use todo_handler::{
    todo_add_handler, todo_create_handler, todo_delete_handler, todo_edit_handler,
    todo_list_handler, todo_patch_handler,
};

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use axum_messages::Messages;
use tower_sessions::Session;

use crate::model::Todo;

/* --------------------------------------- */
/* ------------ region: Utils ------------ */
/* --------------------------------------- */

const FROM_PROTECTED_KEY: &str = "from_protected";
const TZONE_KEY: &str = "time_zone";

/// Handler to check the status of the app.
pub async fn health_checker_handler() -> impl IntoResponse {
    const MESSAGE: &str =
        "Full stack Web App using Rust's Axum framework, Askama, HTMX, JWT & SQLITE3";

    let json_response = serde_json::json!({
        "status": "success",
        "message": MESSAGE
    });

    Json(json_response)
}

/// Set flag in session.
async fn set_flag_in_session(session: &Session, from_protected: bool) {
    session
        .insert(FROM_PROTECTED_KEY, from_protected)
        .await
        .unwrap();
}

/// Set tzone in session.
async fn set_tzone_in_session(session: &Session, tzone: String) {
    session.insert(TZONE_KEY, tzone).await.unwrap();
}

/// Format flash messages generated in redirects.
fn get_messages(messages: Messages) -> (String, String) {
    let mut messages = messages
        .into_iter()
        .map(|message| format!("{}: {}", message.level, message))
        .collect::<Vec<_>>()
        .join(", ");
    let mut messages_status = "".to_string();

    if messages.len() != 0 && messages.contains("Success") {
        messages_status = messages[..7].to_string();
        messages = messages[9..].to_string();
    } else if messages.len() != 0 && messages.contains("Error") {
        messages_status = messages[..5].to_string();
        messages = messages[7..].to_string();
    }

    (messages_status, messages)
}

/// convert_datetime converts the datetime format from the
/// database (UTC timestamp) to a string in RFC822Z format,
/// taking the client's timezone (&str) and a datetime (NaiveDateTime).
pub fn convert_datetime(tzone: &str, dt: NaiveDateTime) -> String {
    let tz = tzone.parse::<Tz>().unwrap();
    let converted = Local.from_utc_datetime(&dt);
    let dttz = converted.with_timezone(&tz).to_rfc2822();

    // conversion to RFC822Z format
    let chars = dttz.chars().collect::<Vec<_>>();
    let first_part = chars[5..22].iter().collect::<String>();
    let last_part = chars[25..].iter().collect::<String>();

    format!("{}{}", first_part, last_part)
}

/* --------------------------------------- */
/* ----------- enregion: Utils ----------- */
/* --------------------------------------- */

/* --------------------------------------- */
/* ------ region: Template Rendering ----- */
/* --------------------------------------- */

/// A wrapper type that we'll use to encapsulate HTML parsed
/// by askama into valid HTML for axum to serve.
struct HtmlTemplate<T>(T);

/// Allows us to convert Askama HTML templates into valid HTML
/// for axum to serve in the response.
impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        // Attempt to render the template with askama
        match self.0.render() {
            // If we're able to successfully parse and aggregate the template, serve it
            Ok(html) => Html(html).into_response(),
            // If we're not, return an error or some bit of fallback HTML
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

/// Home page template
#[derive(Default, Template)]
#[template(path = "auth/home.html")]
struct HomeTemplate {
    title: String,
    username: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Register page template
#[derive(Default, Template)]
#[template(path = "auth/register.html")]
struct RegisterTemplate {
    title: String,
    username: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Login page template
#[derive(Default, Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate {
    title: String,
    username: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Todolist page template
#[derive(Default, Template)]
#[template(path = "todos/todo_list.html")]
struct TodoListTemplate {
    title: String,
    title_page: String,
    username: String,
    todos: Vec<Todo>,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Todo creation todo dialog template
#[derive(Default, Template)]
#[template(path = "partials/todo_creation_modal.html")]
struct TodoCreationModalTemplate;

/// Todo update todo dialog template
#[derive(Default, Template)]
#[template(path = "partials/todo_update_modal.html")]
struct TodoUpdateModalTemplate {
    todo: Todo,
    datetime: String,
    is_error: bool,
    reason: String,
}

/// Error 401 page template
#[derive(Default, Template)]
#[template(path = "error/error_401.html")]
struct Error401Template {
    title: String,
    username: String,
    reason: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Error 404 page template
#[derive(Default, Template)]
#[template(path = "error/error_404.html")]
struct Error404Template {
    title: String,
    username: String,
    reason: String,
    link: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/// Error 500 page template
#[derive(Default, Template)]
#[template(path = "error/error_500.html")]
struct Error500Template {
    title: String,
    username: String,
    reason: String,
    link: String,
    messages_status: String,
    messages: String,
    from_protected: bool,
    is_error: bool,
}

/* --------------------------------------- */
/* ---- endregion: Template Rendering ---- */
/* --------------------------------------- */

/* IMPORTANT!! REGARDING IMPL INTORESPONSE. SEE:
https://docs.rs/axum/latest/axum/response/index.html#regarding-impl-intoresponse
*/

/* IMPORTANT!! A MORE APPROPRIATE WAY TO HANDLE ERRORS. SEE:
https://github.com/tokio-rs/axum/discussions/2446
https://github.com/tokio-rs/axum/blob/main/examples/reqwest-response/src/main.rs
https://docs.rs/axum/latest/axum/error_handling/index.html
*/

/* IMPORTANT!! IN AXUM, THE LAST EXTRACTOR OF A HANDLER CANNOT IMPLEMENT `FromRequestParts`. SEE:
https://docs.rs/axum/latest/axum/extract/index.html#the-order-of-extractors
https://docs.rs/axum/latest/axum/handler/trait.Handler.html#debugging-handler-type-errors
https://docs.rs/axum-macros/latest/axum_macros/attr.debug_handler.html
https://docs.rs/axum/latest/axum/extract/trait.FromRequestParts.html

https://stackoverflow.com/questions/76307624/unexplained-trait-bound-no-longer-satisfied-when-modifying-axum-handler-body
https://github.com/emarifer/axum-postgres-api/blob/main/src/handler.rs#L32-L71
*/

/* CAPITALIZE A STRING IN RUST:
https://nick.groenen.me/notes/capitalize-a-string-in-rust/

/// Capitalizes the first character in s.
// fn capitalize(s: &str) -> String {
//     let mut c = s.chars();
//     match c.next() {
//         None => String::new(),
//         Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
//     }
// }
*/
