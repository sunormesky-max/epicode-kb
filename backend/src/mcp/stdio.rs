//! MCP stdio transport adapter.

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use crate::error::AppResult;
use crate::mcp::server::{McpRequest, McpServer};

/// Run the MCP server over standard input/output.
pub async fn run_stdio_server(server: Arc<McpServer>) -> AppResult<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line.map_err(crate::error::AppError::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let req: McpRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = crate::mcp::server::McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(crate::mcp::server::McpError {
                        code: -32700,
                        message: format!("parse error: {}", e),
                    }),
                };
                let _ = writeln!(stdout_lock, "{}", serde_json::to_string(&resp).unwrap());
                continue;
            }
        };

        let resp = server.handle_request(req).await;
        let json = serde_json::to_string(&resp).unwrap();
        writeln!(stdout_lock, "{}", json).map_err(crate::error::AppError::Io)?;
        stdout_lock.flush().map_err(crate::error::AppError::Io)?;
    }

    Ok(())
}
