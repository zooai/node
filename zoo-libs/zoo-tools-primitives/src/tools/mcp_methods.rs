use rmcp::client::McpClient;
use rmcp::model::{CallToolRequest, CallToolResult};
use rmcp::transport::{ChildProcessTransport, SseClientTransport};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

const MCP_TIMEOUT: Duration = Duration::from_secs(30);

/// Run MCP tool via child process (command)
pub async fn run_tool_via_command(
    command: String,
    tool: String,
    env: HashMap<String, String>,
    parameters: serde_json::Map<String, Value>,
) -> Result<CallToolResult, String> {
    // Parse command into parts
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command provided".to_string());
    }

    let program = parts[0].to_string();
    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    // Create child process transport
    let transport = ChildProcessTransport::new(program, args, env)
        .map_err(|e| format!("Failed to create child process transport: {:?}", e))?;

    // Create MCP client
    let client = McpClient::new(transport);

    // Initialize the client
    timeout(MCP_TIMEOUT, client.initialize("zoo-node".to_string(), "1.0.0".to_string()))
        .await
        .map_err(|_| "Client initialization timed out".to_string())?
        .map_err(|e| format!("Failed to initialize client: {:?}", e))?;

    // Call the tool
    let request = CallToolRequest {
        name: tool,
        arguments: Some(Value::Object(parameters)),
    };

    timeout(MCP_TIMEOUT, client.call_tool(request))
        .await
        .map_err(|_| "Tool call timed out".to_string())?
        .map_err(|e| format!("Tool call failed: {:?}", e))
}

/// Run MCP tool via SSE (Server-Sent Events)
pub async fn run_tool_via_sse(
    url: String,
    tool: String,
    parameters: serde_json::Map<String, Value>,
) -> Result<CallToolResult, String> {
    // Create SSE transport
    let transport = SseClientTransport::new(&url)
        .await
        .map_err(|e| format!("Failed to create SSE transport: {:?}", e))?;

    // Create MCP client
    let client = McpClient::new(transport);

    // Initialize the client
    timeout(MCP_TIMEOUT, client.initialize("zoo-node".to_string(), "1.0.0".to_string()))
        .await
        .map_err(|_| "Client initialization timed out".to_string())?
        .map_err(|e| format!("Failed to initialize client: {:?}", e))?;

    // Call the tool
    let request = CallToolRequest {
        name: tool,
        arguments: Some(Value::Object(parameters)),
    };

    timeout(MCP_TIMEOUT, client.call_tool(request))
        .await
        .map_err(|_| "Tool call timed out".to_string())?
        .map_err(|e| format!("Tool call failed: {:?}", e))
}

/// Run MCP tool via HTTP
pub async fn run_tool_via_http(
    url: String,
    tool: String,
    parameters: serde_json::Map<String, Value>,
) -> Result<CallToolResult, String> {
    // For now, HTTP is handled similar to SSE in rmcp 0.8
    // This may need to be adjusted based on actual HTTP transport implementation
    run_tool_via_sse(url, tool, parameters).await
}
