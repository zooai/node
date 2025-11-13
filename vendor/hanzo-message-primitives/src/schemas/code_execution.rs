use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use super::hanzo_tools::CodeLanguage;

/// Request for code execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct CodeExecutionRequest {
    /// The code to execute
    pub code: String,

    /// Programming language of the code
    pub language: CodeLanguage,

    /// Optional input/stdin for the code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,

    /// Environment variables to set for execution
    #[serde(default)]
    pub env_vars: Vec<EnvVar>,

    /// Command line arguments to pass to the program
    #[serde(default)]
    pub args: Vec<String>,

    /// Maximum execution time in milliseconds (default: 30000 ms = 30s)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Maximum memory limit in bytes (default: 512MB)
    #[serde(default = "default_memory_limit")]
    pub memory_limit: u64,

    /// Whether to run in a sandboxed environment (default: true)
    #[serde(default = "default_sandbox")]
    pub sandbox: bool,

    /// Optional files to make available during execution
    #[serde(default)]
    pub files: Vec<CodeFile>,

    /// Whether to capture and return compilation output
    #[serde(default)]
    pub capture_compile_output: bool,

    /// Whether to stream output as it's generated (for WebSocket connections)
    #[serde(default)]
    pub stream_output: bool,

    /// Optional user identifier for rate limiting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Optional session ID for tracking execution history
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Response from code execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct CodeExecutionResponse {
    /// Unique execution ID
    pub execution_id: String,

    /// Status of the execution
    pub status: ExecutionStatus,

    /// Standard output from the program
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,

    /// Standard error output from the program
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,

    /// Exit code of the program
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,

    /// Compilation output (if applicable and requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_output: Option<String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// Memory used in bytes
    pub memory_used: u64,

    /// Error message if execution failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Language that was executed
    pub language: CodeLanguage,

    /// Timestamp of execution start
    pub started_at: String,

    /// Timestamp of execution completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,

    /// Files generated during execution (if any)
    #[serde(default)]
    pub output_files: Vec<OutputFile>,

    /// Metrics about the execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<ExecutionMetrics>,
}

/// Status of code execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Code is queued for execution
    Queued,
    /// Code is currently being compiled
    Compiling,
    /// Code is currently executing
    Running,
    /// Code executed successfully
    Success,
    /// Code execution failed
    Failed,
    /// Code execution timed out
    TimedOut,
    /// Code execution was killed due to resource limits
    Killed,
    /// Code execution was cancelled
    Cancelled,
}

/// Environment variable for code execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// File to be available during code execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct CodeFile {
    /// File name/path relative to working directory
    pub name: String,
    /// File content
    pub content: String,
    /// Whether this file is executable
    #[serde(default)]
    pub executable: bool,
}

/// Output file generated during execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct OutputFile {
    /// File name/path
    pub name: String,
    /// File content (base64 encoded if binary)
    pub content: String,
    /// Size in bytes
    pub size: u64,
    /// Whether content is base64 encoded
    #[serde(default)]
    pub is_binary: bool,
}

/// Detailed metrics about code execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct ExecutionMetrics {
    /// CPU time used in milliseconds
    pub cpu_time_ms: u64,
    /// Wall clock time in milliseconds
    pub wall_time_ms: u64,
    /// Peak memory usage in bytes
    pub peak_memory: u64,
    /// Number of system calls made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syscall_count: Option<u64>,
    /// Number of context switches
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_switches: Option<u64>,
}

/// Streaming output event for WebSocket connections
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodeExecutionEvent {
    /// Status update
    StatusUpdate {
        status: ExecutionStatus,
        message: Option<String>,
    },
    /// Output from stdout
    StdoutChunk {
        data: String,
    },
    /// Output from stderr
    StderrChunk {
        data: String,
    },
    /// Compilation output
    CompileOutput {
        data: String,
    },
    /// Execution completed
    Complete {
        response: CodeExecutionResponse,
    },
    /// Error occurred
    Error {
        message: String,
        code: Option<String>,
    },
}

// Default functions
fn default_timeout() -> u64 {
    30000 // 30 seconds
}

fn default_memory_limit() -> u64 {
    536870912 // 512MB
}

fn default_sandbox() -> bool {
    true
}

impl Default for CodeExecutionRequest {
    fn default() -> Self {
        Self {
            code: String::new(),
            language: CodeLanguage::Python,
            input: None,
            env_vars: Vec::new(),
            args: Vec::new(),
            timeout_ms: default_timeout(),
            memory_limit: default_memory_limit(),
            sandbox: default_sandbox(),
            files: Vec::new(),
            capture_compile_output: false,
            stream_output: false,
            user_id: None,
            session_id: None,
        }
    }
}

impl ExecutionStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExecutionStatus::Success
                | ExecutionStatus::Failed
                | ExecutionStatus::TimedOut
                | ExecutionStatus::Killed
                | ExecutionStatus::Cancelled
        )
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self,
            ExecutionStatus::Queued | ExecutionStatus::Compiling | ExecutionStatus::Running
        )
    }
}