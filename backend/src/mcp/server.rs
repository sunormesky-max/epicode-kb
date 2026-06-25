//! Model Context Protocol (MCP) server implementation.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppResult;
use crate::mcp::tools::{GetMemoryTool, RememberTool, SearchMemoriesTool};

/// MCP request.
#[derive(Debug, Clone, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// MCP response.
#[derive(Debug, Clone, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// MCP error.
#[derive(Debug, Clone, Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

/// MCP tool trait.
#[async_trait::async_trait]
pub trait McpTool: Send + Sync {
    /// Tool name.
    fn name(&self) -> &'static str;
    /// Tool description.
    fn description(&self) -> &'static str;
    /// JSON schema for the tool input.
    fn schema(&self) -> Value;
    /// Execute the tool with the given arguments.
    async fn execute(&self, args: Value) -> AppResult<Value>;
}

/// MCP server holding registered tools.
pub struct McpServer {
    tools: HashMap<String, Arc<dyn McpTool>>,
}

impl McpServer {
    /// Create a new MCP server with default tools.
    pub fn new(state: Arc<crate::state::AppState>) -> Self {
        let mut tools: HashMap<String, Arc<dyn McpTool>> = HashMap::new();
        tools.insert(
            "search_memories".to_string(),
            Arc::new(SearchMemoriesTool::new(state.clone())),
        );
        tools.insert(
            "remember".to_string(),
            Arc::new(RememberTool::new(state.clone())),
        );
        tools.insert(
            "get_memory".to_string(),
            Arc::new(GetMemoryTool::new(state.clone())),
        );
        Self { tools }
    }

    /// Handle an MCP request and return a response.
    pub async fn handle_request(&self, req: McpRequest) -> McpResponse {
        let id = req.id.clone();
        match req.method.as_str() {
            "initialize" => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "serverInfo": {
                        "name": "epicode-kb-mcp",
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                })),
                error: None,
            },
            "tools/list" => {
                let tools: Vec<Value> = self
                    .tools
                    .values()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name(),
                            "description": t.description(),
                            "inputSchema": t.schema(),
                        })
                    })
                    .collect();
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(serde_json::json!({ "tools": tools })),
                    error: None,
                }
            }
            "tools/call" => match req.params {
                Some(params) => match params.get("name").and_then(|v| v.as_str()) {
                    Some(name) => {
                        if let Some(tool) = self.tools.get(name) {
                            let args = params
                                .get("arguments")
                                .cloned()
                                .unwrap_or_else(|| serde_json::json!({}));
                            match tool.execute(args).await {
                                Ok(result) => McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id,
                                    result: Some(serde_json::json!({
                                        "content": [{"type": "text", "text": result.to_string()}]
                                    })),
                                    error: None,
                                },
                                Err(e) => McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id,
                                    result: None,
                                    error: Some(McpError {
                                        code: -32603,
                                        message: e.to_string(),
                                    }),
                                },
                            }
                        } else {
                            McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result: None,
                                error: Some(McpError {
                                    code: -32601,
                                    message: format!("tool not found: {}", name),
                                }),
                            }
                        }
                    }
                    None => McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(McpError {
                            code: -32602,
                            message: "missing tool name".to_string(),
                        }),
                    },
                },
                None => McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "missing params".to_string(),
                    }),
                },
            },
            _ => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32601,
                    message: format!("method not found: {}", req.method),
                }),
            },
        }
    }
}
