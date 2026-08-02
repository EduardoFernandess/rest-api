mod auth;
mod error;
mod middleware;
mod models;
mod routes;
mod state;

use anyhow::Result;
use axum::middleware as axum_mw;
use axum::routing::{get, post};
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/restapi".into());
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-me".into());
    let rate_limit = std::env::var("RATE_LIMIT_PER_MINUTE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState::new(pool, jwt_secret, rate_limit);

    let public = Router::new()
        .route("/health", get(routes::health))
        .route("/auth/register", post(routes::register))
        .route("/auth/login", post(routes::login));

    let protected = Router::new()
        .route("/items", get(routes::list_items).post(routes::create_item))
        .route(
            "/items/{id}",
            get(routes::get_item)
                .put(routes::update_item)
                .delete(routes::delete_item),
        )
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    let app = Router::new()
        .merge(public)
        .merge(protected)
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::rate_limit,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".into())
        .parse()?;
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
