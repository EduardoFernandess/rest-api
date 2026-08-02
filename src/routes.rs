use crate::auth::{hash_password, issue_token, verify_password, AuthUser};
use crate::error::{AppError, AppResult};
use crate::models::{
    AuthResponse, CreateItemRequest, Item, LoginRequest, RegisterRequest, UpdateItemRequest, User,
};
use crate::state::AppState;
use axum::extract::{Extension, Path, State};
use axum::Json;
use uuid::Uuid;

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    if payload.email.is_empty() || payload.password.len() < 8 {
        return Err(AppError::BadRequest(
            "email required and password must be >= 8 chars".into(),
        ));
    }
    let password_hash = hash_password(&payload.password)?;
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash)
        VALUES ($1, $2)
        RETURNING id, email, password_hash, created_at
        "#,
    )
    .bind(&payload.email)
    .bind(&password_hash)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.constraint().is_some() => {
            AppError::Conflict("email already registered".into())
        }
        other => AppError::Sqlx(other),
    })?;

    let token = issue_token(user.id, &state.jwt_secret)?;
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM users WHERE email = $1",
    )
    .bind(&payload.email)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !verify_password(&payload.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let token = issue_token(user.id, &state.jwt_secret)?;
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
    }))
}

pub async fn list_items(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<Item>>> {
    let items = sqlx::query_as::<_, Item>(
        r#"
        SELECT id, user_id, title, body, created_at, updated_at
        FROM items WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(items))
}

pub async fn create_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<CreateItemRequest>,
) -> AppResult<Json<Item>> {
    if payload.title.is_empty() {
        return Err(AppError::BadRequest("title required".into()));
    }
    let item = sqlx::query_as::<_, Item>(
        r#"
        INSERT INTO items (user_id, title, body)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, title, body, created_at, updated_at
        "#,
    )
    .bind(auth.user_id)
    .bind(&payload.title)
    .bind(&payload.body)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(item))
}

pub async fn get_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Item>> {
    let item = sqlx::query_as::<_, Item>(
        r#"
        SELECT id, user_id, title, body, created_at, updated_at
        FROM items WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(item))
}

pub async fn update_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateItemRequest>,
) -> AppResult<Json<Item>> {
    let existing = sqlx::query_as::<_, Item>(
        "SELECT id, user_id, title, body, created_at, updated_at FROM items WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let title = payload.title.unwrap_or(existing.title);
    let body = payload.body.unwrap_or(existing.body);

    let item = sqlx::query_as::<_, Item>(
        r#"
        UPDATE items SET title = $1, body = $2, updated_at = NOW()
        WHERE id = $3 AND user_id = $4
        RETURNING id, user_id, title, body, created_at, updated_at
        "#,
    )
    .bind(title)
    .bind(body)
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(item))
}

pub async fn delete_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM items WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}
