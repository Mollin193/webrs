use axum::Router;

use crate::app::AppState;

mod auth;
mod user;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .nest(
            "/api",
            Router::new()
                .nest("/users", user)
        )
}
