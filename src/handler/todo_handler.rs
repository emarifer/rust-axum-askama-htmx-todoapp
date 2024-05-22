use std::sync::Arc;

use askama::filters::capitalize;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Extension, Form,
};
use axum_messages::Messages;
use serde::Deserialize;
use tokio::sync::RwLock;
use tower_sessions::Session;

use crate::{
    model::{TodoEditSchema, TodoSchema, User},
    service::{add_todo, get_todo_by_id, remove_todo, update_todo},
    AppState,
};

use super::{
    convert_datetime, get_messages, Error400Template, Error404Template, Error500Template,
    HtmlTemplate, TodoCreationModalTemplate, TodoListTemplate, TodoUpdateModalTemplate,
    FROM_PROTECTED_KEY, TZONE_KEY,
};

/// Struct for holding the todo_id (i64) that comes in query params.
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub id: i64,
}

/// Handler to serve the Todo List Page template.
pub async fn todo_list_handler(
    Extension(user): Extension<User>,
    State(state): State<Arc<RwLock<AppState>>>,
    messages: Messages,
    session: Session,
) -> impl IntoResponse {
    let from_protected: bool = session
        .get(FROM_PROTECTED_KEY)
        .await
        .unwrap()
        .unwrap_or_default();

    let (messages_status, messages) = get_messages(messages);

    let full_title = format!(
        "{}'s Task List",
        capitalize(&user.username).unwrap_or_else(|_| user.username.to_owned())
    );

    let lock = state.read().await;
    let todos = lock.todos.clone();
    drop(lock);

    HtmlTemplate(TodoListTemplate {
        title: full_title.to_owned(),
        title_page: full_title,
        username: user.username,
        todos,
        messages_status,
        messages,
        from_protected,
        ..Default::default()
    })
    .into_response()
}

/// Handler to show the Todo Create Modal template.
pub async fn todo_create_handler() -> impl IntoResponse {
    HtmlTemplate(TodoCreationModalTemplate)
}

/// Handle the `POST` request to create a new Todo.
pub async fn todo_add_handler(
    Extension(user): Extension<User>,
    messages: Messages,
    State(state): State<Arc<RwLock<AppState>>>,
    Form(form_data): Form<TodoSchema>,
) -> impl IntoResponse {
    if form_data.title.trim() == "" {
        return HtmlTemplate(Error400Template {
            title: "Error 400".to_string(),
            reason: "You must enter at least one title for the Todo".to_string(),
            is_error: true,
            ..Default::default()
        })
        .into_response();
    }

    let lock = state.read().await;

    match add_todo(user.id, form_data.title, form_data.description, &lock.pool).await {
        Ok(todo) => {
            drop(lock);
            let mut lock = state.write().await;
            lock.todos.insert(0, todo);
            drop(lock);

            messages.success("Task created successfully!!");

            Redirect::to("/todo/list").into_response()
        }
        Err(e) => HtmlTemplate(Error500Template {
            title: "Error 500".to_string(),
            reason: e.to_string(),
            is_error: true,
            link: "/todo/list".to_string(),
            ..Default::default()
        })
        .into_response(),
    }
}

/// Handler to show the Todo Edit Modal template.
pub async fn todo_edit_handler(
    Query(QueryParams { id }): Query<QueryParams>,
    session: Session,
    State(state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    let lock = state.read().await;

    let result = get_todo_by_id(id, &lock.pool).await;
    drop(lock);

    let mut lock = state.write().await;
    if let Err(e) = result {
        lock.todos.retain(|item| item.id != id);
        drop(lock);

        return HtmlTemplate(TodoUpdateModalTemplate {
            is_error: true,
            reason: e.to_string(),
            ..Default::default()
        });
    }

    let tzone: String = session.get(TZONE_KEY).await.unwrap().unwrap_or_default();
    let todo = result.unwrap();
    let datetime = convert_datetime(&tzone, todo.created_at);

    HtmlTemplate(TodoUpdateModalTemplate {
        todo,
        datetime,
        ..Default::default()
    })
}

/// Handle the `POST` (`PATCH`) request to edit a Todo.
pub async fn todo_patch_handler(
    Query(QueryParams { id }): Query<QueryParams>,
    messages: Messages,
    State(state): State<Arc<RwLock<AppState>>>,
    Form(form_data): Form<TodoEditSchema>,
) -> impl IntoResponse {
    if form_data.title.trim() == "" {
        return HtmlTemplate(Error400Template {
            title: "Error 400".to_string(),
            reason: "You must enter at least one title for the Todo".to_string(),
            is_error: true,
            ..Default::default()
        })
        .into_response();
    }

    let lock = state.read().await;

    let result = update_todo(
        form_data.title.clone(),
        form_data.description.clone(),
        form_data.hidden.parse::<bool>().unwrap_or_default(),
        id,
        &lock.pool,
    )
    .await;
    drop(lock);

    let mut lock = state.write().await;
    if let Err(e) = result {
        lock.todos.retain(|item| item.id != id);

        return HtmlTemplate(Error404Template {
            title: "Error 404".to_string(),
            reason: e.to_string(),
            link: "/todo/list".to_string(),
            is_error: true,
            ..Default::default()
        })
        .into_response();
    }

    let index = lock.todos.iter().position(|item| item.id == id).unwrap();
    lock.todos[index].title = form_data.title;
    lock.todos[index].description = form_data.description;
    lock.todos[index].status = form_data.hidden.parse::<bool>().unwrap_or_default();
    drop(lock);

    messages.success("Task successfully updated!!");

    Redirect::to("/todo/list").into_response()
}

/// Handle the `DELETE` request to remove a Todo.
pub async fn todo_delete_handler(
    Query(QueryParams { id }): Query<QueryParams>,
    messages: Messages,
    State(state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    let lock = state.read().await;

    match remove_todo(id, &lock.pool).await {
        Ok(_) => {
            drop(lock);
            let mut lock = state.write().await;
            lock.todos.retain(|item| item.id != id);
            // lock.todos = lock
            //     .todos
            //     .clone()
            //     .into_iter()
            //     .filter(|item| item.id != id)
            //     .collect();
            drop(lock);

            messages.success("Task successfully deleted!!");

            Redirect::to("/todo/list").into_response()
        }
        Err(e) => {
            drop(lock);
            let mut lock = state.write().await;
            lock.todos.retain(|item| item.id != id);
            drop(lock);

            HtmlTemplate(Error404Template {
                title: "Error 404".to_string(),
                reason: e.to_string(),
                link: "/todo/list".to_string(),
                is_error: true,
                ..Default::default()
            })
            .into_response()
        }
    }
}

/* REFERENCES 22-05-2024:
https://www.youtube.com/@_noisecode/videos
https://dev.to/pongsakornsemsuwan/rust-axum-extracting-query-param-of-vec-4pdm
https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/checkbox
https://www.google.com/search?q=askama+how+get+checkbox&oq=askama+how+get+che&aqs=chrome.1.69i57j33i160l5.12476j0j4&sourceid=chrome&ie=UTF-8

https://github.com/yarox/todomvc
*/

/* REFERENCES 21-05-2024:
https://github.com/yarox/todomvc/blob/main/templates/components/todo/item.html
https://medium.com/intelliconnect-engineering/step-by-step-guide-to-test-driven-development-tdd-in-rust-axum-5bef05fd7366
*/

/* REFERENCES 16-05-2024:
https://github.com/tokio-rs/axum/discussions/629
https://github.com/tokio-rs/axum/blob/dea36db400f27c025b646e5720b9a6784ea4db6e/examples/key-value-store/src/main.rs
https://stackoverflow.com/questions/26243025/how-to-remove-an-element-from-a-vector-given-the-element
https://stackoverflow.com/questions/44662312/how-to-filter-a-vector-of-custom-structs

https://docs.rs/axum/latest/axum/handler/index.html#debugging-handler-type-errors
https://docs.rs/axum-macros/latest/axum_macros/attr.debug_handler.html
https://docs.rs/sqlx/latest/sqlx/macro.query_as.html#
https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html#which-kind-of-mutex-should-you-use
https://stackoverflow.com/questions/73840520/what-is-the-difference-between-stdsyncmutex-vs-tokiosyncmutex
https://stackoverflow.com/questions/50704279/when-or-why-should-i-use-a-mutex-over-an-rwlock
https://github.com/tokio-rs/axum/discussions/629
https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html
https://medium.com/@ztroop/introduction-to-arc-in-rust-3174e65a0aab

https://rust-classes.com/preface
https://www.propelauth.com/post/clean-code-with-rust-and-axum
https://github.com/leotaku/tower-livereload
*/

/*
// let mut headers = HeaderMap::new();
// headers.insert("HX-Trigger-After-Swap", "my-event".parse().unwrap());

// hx-on:my-event="setTimeout(() => location.reload(), 300)"

// // HtmlTemplate(TodoItemListTemplate { todo }).into_response()
// (headers, HtmlTemplate(TodoItemListTemplate { todo })).into_response()

// HtmlTemplate(TodoCreateTemplate {
//     title: "Create Todo".to_string(),
//     username: user.username,
//     button_text: "✅  Ok".to_string(),
//     from_protected,
//     ..Default::default()
// })
// .into_response()
*/
