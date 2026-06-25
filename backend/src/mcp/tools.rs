//! MCP tools: search_memories, remember, get_memory.

use std::sync::Arc;

use serde_json::Value;

use crate::auth::model::AgentContext;
use crate::error::{AppError, AppResult};
use crate::mcp::server::McpTool;
use crate::memory::model::{CreateMemoryRequest, Provenance, Visibility};
use crate::memory::service::MemoryService;
use crate::search::{SearchMode, SearchQuery};
use crate::state::AppState;

/// Tool to search memories.
pub struct SearchMemoriesTool {
    state: Arc<AppState>,
}

impl SearchMemoriesTool {
    /// Create a new search memories tool.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl McpTool for SearchMemoriesTool {
    fn name(&self) -> &'static str {
        "search_memories"
    }

    fn description(&self) -> &'static str {
        "Search memories in a space using hybrid semantic + full-text search."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "required": ["q", "space_id"],
            "properties": {
                "q": {"type": "string", "description": "Search query"},
                "space_id": {"type": "string", "description": "Space ID"},
                "limit": {"type": "integer", "default": 10},
            }
        })
    }

    async fn execute(&self, args: Value) -> AppResult<Value> {
        let q = args["q"]
            .as_str()
            .ok_or_else(|| AppError::bad_request("missing q"))?;
        let space_id = args["space_id"]
            .as_str()
            .ok_or_else(|| AppError::bad_request("missing space_id"))?;
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;

        let query = SearchQuery {
            q: q.to_string(),
            space_id: space_id.to_string(),
            mode: SearchMode::Hybrid,
            min_trust: None,
            provenance: None,
            review_status: None,
            visibility: None,
            limit,
            offset: 0,
        };

        let response = self.state.search_engine.search(&query)?;
        Ok(serde_json::to_value(response)?)
    }
}

/// Tool to create a memory.
pub struct RememberTool {
    state: Arc<AppState>,
}

impl RememberTool {
    /// Create a new remember tool.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl McpTool for RememberTool {
    fn name(&self) -> &'static str {
        "remember"
    }

    fn description(&self) -> &'static str {
        "Store a new memory in the knowledge base. The memory will be marked as AI-generated and pending review."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "required": ["space_id", "content"],
            "properties": {
                "space_id": {"type": "string"},
                "content": {"type": "string"},
                "provenance_meta": {"type": "object"},
            }
        })
    }

    async fn execute(&self, args: Value) -> AppResult<Value> {
        let space_id = args["space_id"]
            .as_str()
            .ok_or_else(|| AppError::bad_request("missing space_id"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| AppError::bad_request("missing content"))?;

        let service = MemoryService::from_state(&self.state);

        // MCP remember acts as an agent write with a synthetic agent context.
        let agent = AgentContext {
            api_key_id: "mcp".to_string(),
            space_id: space_id.to_string(),
            user_id: "mcp".to_string(),
            user_role: crate::auth::model::GlobalRole::Editor,
            space_role: Some(crate::auth::model::SpaceRole::Editor),
            scope: "write".to_string(),
        };

        let req = CreateMemoryRequest {
            space_id: space_id.to_string(),
            content: content.to_string(),
            provenance: Provenance::Ai,
            trust_level: None,
            provenance_meta: args.get("provenance_meta").cloned(),
            review_status: None,
            visibility: Some(Visibility::Inherit),
        };

        let memory = service.create_from_agent(req, &agent)?;
        Ok(serde_json::json!({
            "id": memory.id,
            "review_status": memory.review_status.as_str(),
        }))
    }
}

/// Tool to get a single memory.
pub struct GetMemoryTool {
    state: Arc<AppState>,
}

impl GetMemoryTool {
    /// Create a new get memory tool.
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl McpTool for GetMemoryTool {
    fn name(&self) -> &'static str {
        "get_memory"
    }

    fn description(&self) -> &'static str {
        "Get a single memory by ID."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": {"type": "string"},
            }
        })
    }

    async fn execute(&self, args: Value) -> AppResult<Value> {
        let id = args["id"]
            .as_str()
            .ok_or_else(|| AppError::bad_request("missing id"))?;
        let service = MemoryService::from_state(&self.state);
        let memory = service.get_by_id(id)?;
        Ok(serde_json::to_value(memory)?)
    }
}
