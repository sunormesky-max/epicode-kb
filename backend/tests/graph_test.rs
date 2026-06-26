//! Integration tests for the knowledge graph endpoint (P3-4 conflict edges).

mod common;

use std::sync::Arc;

use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use epicode_kb::auth::model::{CreateLocalUserRequest, GlobalRole, LoginRequest};
use epicode_kb::db::repository::MemoryRepo;
use epicode_kb::memory::model::{Memory, Provenance, ReviewStatus};
use epicode_kb::now_ts;

/// Insert an accepted human memory node.
fn seed_node(state: &Arc<epicode_kb::state::AppState>, id: &str, content: &str) {
    let mut mem = Memory::new("sp_default".to_string(), content.to_string(), Provenance::Human);
    mem.id = id.to_string();
    mem.review_status = ReviewStatus::Accepted;
    let conn = state.db.lock().unwrap();
    MemoryRepo::insert(&conn, &mem).unwrap();
}

/// Insert a conflict-provenance memory linking two source nodes.
fn seed_conflict(
    state: &Arc<epicode_kb::state::AppState>,
    cf_id: &str,
    a: &str,
    b: &str,
) {
    let now = now_ts();
    let mut mem = Memory::new("sp_default".to_string(), "conflict report".to_string(), Provenance::Conflict);
    mem.id = cf_id.to_string();
    mem.created_at = now;
    mem.updated_at = now;
    mem.provenance_meta = Some(serde_json::json!({
        "conflicting_ids": [a, b],
        "confidence": 0.82,
    }));
    let conn = state.db.lock().unwrap();
    MemoryRepo::insert(&conn, &mem).unwrap();
}

#[tokio::test]
async fn test_graph_returns_nodes_and_conflict_edges() {
    let (app, state, _temp) = common::create_test_app().await;

    // Seed nodes + a conflict edge between two of them.
    seed_node(&state, "node_a", "Alpha knowledge statement here");
    seed_node(&state, "node_b", "Beta knowledge statement here");
    seed_node(&state, "node_c", "Gamma knowledge statement here");
    seed_conflict(&state, "cf_1", "node_a", "node_b");

    // Register + login to obtain a valid access token.
    state
        .auth_service
        .register(CreateLocalUserRequest {
            email: "graph@example.com".to_string(),
            name: "Graph".to_string(),
            password: "password123".to_string(),
            global_role: GlobalRole::Admin,
        })
        .unwrap();
    let (tokens, _user) = state
        .auth_service
        .login(LoginRequest {
            email: "graph@example.com".to_string(),
            password: "password123".to_string(),
        })
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?space_id=sp_default")
                .header("Authorization", format!("Bearer {}", tokens.access_token))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["code"], 0, "graph should succeed");

    let nodes = json["data"]["nodes"].as_array().expect("nodes array");
    let node_ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(node_ids.contains(&"node_a"), "nodes should include node_a: {:?}", node_ids);
    assert!(node_ids.contains(&"node_b"));
    assert!(node_ids.contains(&"node_c"));

    let edges = json["data"]["edges"].as_array().expect("edges array");
    let has_conflict_edge = edges.iter().any(|e| {
        let t = e["type"].as_str().unwrap_or("");
        let s = e["source"].as_str().unwrap_or("");
        let d = e["target"].as_str().unwrap_or("");
        t == "conflict"
            && ((s == "node_a" && d == "node_b") || (s == "node_b" && d == "node_a"))
    });
    assert!(
        has_conflict_edge,
        "expected a conflict edge between node_a and node_b, got: {}",
        serde_json::to_string(&edges).unwrap()
    );
}
