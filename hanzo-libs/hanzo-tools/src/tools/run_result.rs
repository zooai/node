use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What a tool invocation returned. One field, because that is all any caller
/// reads. It lives here so the type a tool interface speaks does not depend on
/// which engine, if any, executes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub data: Value,
}
