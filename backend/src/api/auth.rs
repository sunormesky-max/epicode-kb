//! Auth API endpoints: login, refresh, register, spaces, members.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::ApiResponse;
use crate::auth::model::{
    Actor, CreateLocalUserRequest, LoginRequest, RefreshRequest, Space, SpaceMember, SpaceRole,
    SpaceVisibility, User,
};
use crate::db::repository::{SpaceMemberRepo, SpaceRepo, UserRepo};
use crate::error::AppError;
use crate::state::AppState;

/// POST /api/v1/auth/register — register a local user.
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateLocalUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    let user = state.auth_service.register(req)?;
    Ok(Json(ApiResponse::ok(UserResponse::from(&user))))
}

/// POST /api/v1/auth/login — login with email/password.
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    let (tokens, user) = state.auth_service.login(req)?;
    Ok(Json(ApiResponse::ok(LoginResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        user: UserResponse::from(&user),
    })))
}

/// POST /api/v1/auth/refresh — refresh access token.
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<ApiResponse<TokenPairResponse>>, AppError> {
    let tokens = state.auth_service.refresh(req)?;
    Ok(Json(ApiResponse::ok(TokenPairResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
    })))
}

/// GET /api/v1/auth/me — current user info.
pub async fn me(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    let conn = state.db.lock().unwrap();
    let user = UserRepo::get_by_id(&conn, &actor.user_id)?;
    Ok(Json(ApiResponse::ok(UserResponse::from(&user))))
}

/// GET /api/v1/spaces — list spaces.
pub async fn list_spaces(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListSpacesQuery>,
) -> Result<Json<ApiResponse<ListSpacesResponse>>, AppError> {
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);
    let conn = state.db.lock().unwrap();
    let (spaces, total) = SpaceRepo::list(&conn, limit, offset)?;
    Ok(Json(ApiResponse::ok(ListSpacesResponse { spaces, total })))
}

/// POST /api/v1/spaces — create a space.
pub async fn create_space(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<CreateSpaceRequest>,
) -> Result<Json<ApiResponse<Space>>, AppError> {
    let slug = body
        .slug
        .unwrap_or_else(|| body.name.to_lowercase().replace(' ', "-"));

    let space = Space::new(body.name, slug, actor.user_id.clone());

    let conn = state.db.lock().unwrap();
    SpaceRepo::insert(&conn, &space)?;

    // Add creator as owner.
    let member = SpaceMember {
        id: crate::generate_id("sm"),
        space_id: space.id.clone(),
        user_id: actor.user_id.clone(),
        role: SpaceRole::Owner,
        created_at: crate::now_ts(),
    };
    SpaceMemberRepo::upsert(&conn, &member)?;

    Ok(Json(ApiResponse::ok(space)))
}

/// GET /api/v1/spaces/:id — get space details.
pub async fn get_space(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Space>>, AppError> {
    let conn = state.db.lock().unwrap();
    let space = SpaceRepo::get_by_id(&conn, &id)?;
    Ok(Json(ApiResponse::ok(space)))
}

/// PUT /api/v1/spaces/:id/visibility — update space visibility.
pub async fn update_space_visibility(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<UpdateSpaceVisibilityRequest>,
) -> Result<Json<ApiResponse<Space>>, AppError> {
    state.auth_service.rbac().check(
        &crate::auth::rbac::AuthContext {
            user_id: actor.user_id.clone(),
            global_role: actor.global_role,
            space_id: id.clone(),
            space_role: actor.space_role,
        },
        crate::auth::model::Permission::SpaceAdmin,
    )?;

    let now = crate::now_ts();
    let conn = state.db.lock().unwrap();
    SpaceRepo::update_visibility(&conn, &id, body.visibility, now)?;
    let space = SpaceRepo::get_by_id(&conn, &id)?;
    Ok(Json(ApiResponse::ok(space)))
}

/// GET /api/v1/spaces/:id/members — list space members.
pub async fn list_space_members(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<ListMembersQuery>,
) -> Result<Json<ApiResponse<ListMembersResponse>>, AppError> {
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);
    let conn = state.db.lock().unwrap();
    let (members, total) = SpaceMemberRepo::list_by_space(&conn, &id, limit, offset)?;
    Ok(Json(ApiResponse::ok(ListMembersResponse {
        members,
        total,
    })))
}

/// POST /api/v1/spaces/:id/members — invite a member.
pub async fn invite_space_member(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<InviteMemberRequest>,
) -> Result<Json<ApiResponse<SpaceMember>>, AppError> {
    state.auth_service.rbac().check(
        &crate::auth::rbac::AuthContext {
            user_id: actor.user_id.clone(),
            global_role: actor.global_role,
            space_id: id.clone(),
            space_role: actor.space_role,
        },
        crate::auth::model::Permission::SpaceAdmin,
    )?;

    let conn = state.db.lock().unwrap();
    let user = UserRepo::find_by_email(&conn, &body.email)?
        .ok_or_else(|| AppError::not_found(format!("user not found: {}", body.email)))?;

    let member = SpaceMember {
        id: crate::generate_id("sm"),
        space_id: id,
        user_id: user.id,
        role: body.role,
        created_at: crate::now_ts(),
    };
    SpaceMemberRepo::upsert(&conn, &member)?;

    Ok(Json(ApiResponse::ok(member)))
}

// ============================================================
// Response / request DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub global_role: String,
    pub is_active: bool,
    pub created_at: i64,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.clone(),
            email: user.email.clone(),
            name: user.name.clone(),
            global_role: user.global_role.as_str().to_string(),
            is_active: user.is_active,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSpaceRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSpaceVisibilityRequest {
    pub visibility: SpaceVisibility,
}

#[derive(Debug, Deserialize)]
pub struct ListSpacesQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListSpacesResponse {
    pub spaces: Vec<Space>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListMembersQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListMembersResponse {
    pub members: Vec<SpaceMember>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    pub email: String,
    pub role: SpaceRole,
}
