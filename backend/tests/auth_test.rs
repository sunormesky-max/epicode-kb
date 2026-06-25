//! Tests for authentication, RBAC, and JWT.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

mod common;

use epicode_kb::auth::model::{CreateLocalUserRequest, GlobalRole, LoginRequest, Permission, SpaceRole, User};
use epicode_kb::auth::rbac::{AuthContext, RbacEngine};
use epicode_kb::auth::service::hash_api_key;

// ============================================================
// API key hashing
// ============================================================

#[test]
fn test_hash_api_key_deterministic() {
    let h1 = hash_api_key("ak_abc123", Some("salt"));
    let h2 = hash_api_key("ak_abc123", Some("salt"));
    let h3 = hash_api_key("ak_abc123", Some("other"));
    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
}

// ============================================================
// RBAC engine
// ============================================================

fn ctx(role: GlobalRole, space_role: Option<SpaceRole>, space_id: &str) -> AuthContext {
    AuthContext {
        user_id: "usr_test".to_string(),
        global_role: role,
        space_id: space_id.to_string(),
        space_role,
    }
}

#[test]
fn test_rbac_global_admin_has_all_permissions() {
    let rbac = RbacEngine::new();
    for perm in [
        Permission::SpaceRead,
        Permission::SpaceWrite,
        Permission::SpaceAdmin,
        Permission::MemoryRead,
        Permission::MemoryWrite,
        Permission::MemoryAdmin,
        Permission::AgentWrite,
        Permission::ApiKeyManage,
        Permission::UserManage,
    ] {
        assert!(rbac.check(&ctx(GlobalRole::Admin, None, "sp_test"), perm).is_ok());
    }
}

#[test]
fn test_rbac_global_viewer_can_only_read() {
    let rbac = RbacEngine::new();
    assert!(rbac.check(&ctx(GlobalRole::Viewer, None, "sp_test"), Permission::SpaceRead).is_ok());
    assert!(rbac.check(&ctx(GlobalRole::Viewer, None, "sp_test"), Permission::MemoryRead).is_ok());
    assert!(rbac.check(&ctx(GlobalRole::Viewer, None, "sp_test"), Permission::SpaceWrite).is_err());
    assert!(rbac.check(&ctx(GlobalRole::Viewer, None, "sp_test"), Permission::MemoryWrite).is_err());
}

#[test]
fn test_rbac_global_editor_has_space_and_memory_write() {
    let rbac = RbacEngine::new();
    assert!(rbac.check(&ctx(GlobalRole::Editor, None, "sp_test"), Permission::SpaceWrite).is_ok());
    assert!(rbac.check(&ctx(GlobalRole::Editor, None, "sp_test"), Permission::MemoryWrite).is_ok());
    // Per architecture design v2 section 7.2, global Editor should NOT manage users.
    assert!(rbac.check(&ctx(GlobalRole::Editor, None, "sp_test"), Permission::UserManage).is_err());
}

#[test]
fn test_rbac_global_editor_should_not_have_agent_write() {
    // Architecture design v2 section 7.2 maps GlobalRole::Editor to SpaceRead/SpaceWrite/MemoryRead/MemoryWrite only.
    let rbac = RbacEngine::new();
    assert!(rbac.check(&ctx(GlobalRole::Editor, None, "sp_test"), Permission::AgentWrite).is_err());
}

#[test]
fn test_rbac_space_owner_can_manage_api_keys() {
    let rbac = RbacEngine::new();
    assert!(
        rbac.check(
            &ctx(GlobalRole::Viewer, Some(SpaceRole::Owner), "sp_test"),
            Permission::ApiKeyManage,
        )
        .is_ok()
    );
}

#[test]
fn test_rbac_space_viewer_cannot_write() {
    let rbac = RbacEngine::new();
    let ctx = ctx(GlobalRole::Viewer, Some(SpaceRole::Viewer), "sp_test");
    assert!(rbac.check(&ctx, Permission::SpaceRead).is_ok());
    assert!(rbac.check(&ctx, Permission::MemoryRead).is_ok());
    assert!(rbac.check(&ctx, Permission::SpaceWrite).is_err());
    assert!(rbac.check(&ctx, Permission::MemoryWrite).is_err());
    assert!(rbac.check(&ctx, Permission::AgentWrite).is_err());
}

// ============================================================
// Auth service
// ============================================================

#[tokio::test]
async fn test_auth_service_register_and_login() {
    let (_app, state, _temp) = common::create_test_app().await;

    let req = CreateLocalUserRequest {
        email: "alice@example.com".to_string(),
        name: "Alice".to_string(),
        password: "password123".to_string(),
        global_role: GlobalRole::Editor,
    };
    let user = state.auth_service.register(req).unwrap();
    assert_eq!(user.email, "alice@example.com");
    assert!(user.is_active);

    let login = LoginRequest {
        email: "alice@example.com".to_string(),
        password: "password123".to_string(),
    };
    let (tokens, logged_in) = state.auth_service.login(login).unwrap();
    assert_eq!(logged_in.id, user.id);
    assert!(!tokens.access_token.is_empty());

    let claims = state.auth_service.jwt().verify_access(&tokens.access_token).unwrap();
    assert_eq!(claims.sub, user.id);
    assert_eq!(claims.global_role, GlobalRole::Editor);
}

#[tokio::test]
async fn test_auth_service_invalid_password_fails() {
    let (_app, state, _temp) = common::create_test_app().await;

    let req = CreateLocalUserRequest {
        email: "bob@example.com".to_string(),
        name: "Bob".to_string(),
        password: "password123".to_string(),
        global_role: GlobalRole::Viewer,
    };
    state.auth_service.register(req).unwrap();

    let bad_login = LoginRequest {
        email: "bob@example.com".to_string(),
        password: "wrongpassword".to_string(),
    };
    assert!(state.auth_service.login(bad_login).is_err());
}

// ============================================================
// Route-level auth checks
// ============================================================

#[tokio::test]
async fn test_auth_register_route_is_public() {
    let (mut app, _state, _temp) = common::create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(common::json_body(
                    r#"{"email":"public@example.com","name":"Public","password":"password123","global_role":"viewer"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "registration endpoint should be publicly accessible"
    );
}

#[tokio::test]
async fn test_auth_login_route_is_public() {
    let (mut app, state, _temp) = common::create_test_app().await;

    // Seed a user directly via the auth service.
    let req = CreateLocalUserRequest {
        email: "login@example.com".to_string(),
        name: "Login".to_string(),
        password: "password123".to_string(),
        global_role: GlobalRole::Viewer,
    };
    state.auth_service.register(req).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(common::json_body(
                    r#"{"email":"login@example.com","password":"password123"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "login endpoint should be publicly accessible"
    );
}
