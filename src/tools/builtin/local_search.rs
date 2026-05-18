//! Local file search tool with configurable scope gate.
//!
//! Implements the `search_files` action. By default scope is limited to the
//! active workspace; global filesystem search must be explicitly enabled via
//! `LocalSearchSettings.allow_global_scope`.

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;

use ironclaw_safety::sensitive_paths::is_sensitive_path;

use crate::context::JobContext;
use crate::settings::LocalSearchSettings;
use crate::tools::builtin::path_utils::validate_path;
use crate::tools::builtin::shell::SAFE_ENV_VARS;
use crate::tools::tool::{
    ApprovalRequirement, Tool, ToolDiscoverySummary, ToolDomain, ToolError, ToolOutput, require_str,
};

const MAX_OUTPUT_SIZE: usize = 64 * 1024;
const DEFAULT_HEAD_LIMIT: usize = 250;

/// Built-in local file search tool backed by ripgrep.
///
/// When `allow_global_scope` is false (default), the search is confined to the
/// base workspace directory. The LLM receives a clear error if it attempts to
/// escape the workspace with `scope: "global"`.
#[derive(Debug, Default)]
pub struct LocalSearchTool {
    settings: LocalSearchSettings,
    base_dir: Option<PathBuf>,
}

impl LocalSearchTool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_settings(mut self, settings: LocalSearchSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn with_base_dir(mut self, dir: PathBuf) -> Self {
        self.base_dir = Some(dir);
        self
    }
}

fn safe_env() -> std::collections::HashMap<String, String> {
    let mut env = std::collections::HashMap::new();
    for &key in SAFE_ENV_VARS {
        if let Ok(val) = std::env::var(key) {
            env.insert(key.to_string(), val);
        }
    }
    env
}

#[async_trait]
impl Tool for LocalSearchTool {
    fn name(&self) -> &str {
        "local_search"
    }

    fn description(&self) -> &str {
        "Search files in the local workspace using regex patterns. \
         Scope is limited to the active workspace by default. \
         Use this instead of shell with 'grep' or 'rg' for local file searches."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (defaults to workspace root)"
                },
                "scope": {
                    "type": "string",
                    "enum": ["workspace", "global"],
                    "description": "Search scope: 'workspace' (default, safe) or 'global' (requires setting enabled in Settings -> Tools -> Local Search)"
                },
                "glob": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. '*.rs', '*.{ts,tsx}')"
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode: content (matching lines), files_with_matches (paths only, default), count (match counts)"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive search (default false)"
                },
                "head_limit": {
                    "type": "integer",
                    "description": "Maximum output lines/entries (default 250, pass 0 for unlimited)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let start = std::time::Instant::now();

        let pattern = require_str(&params, "pattern")?;
        let scope = params
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or("workspace");
        let glob_filter = params.get("glob").and_then(|v| v.as_str());
        let output_mode = params
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("files_with_matches");
        let case_insensitive = params
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let head_limit = params
            .get("head_limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        if scope == "global" && !self.settings.allow_global_scope {
            return Err(ToolError::NotAuthorized(
                "Global filesystem search is disabled. Enable it in Settings -> Tools -> Local Search."
                    .to_string(),
            ));
        }

        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let search_path = if scope == "global" {
            PathBuf::from(path_str)
        } else {
            let base = self.base_dir.clone().unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });
            validate_path(path_str, Some(&base))?
        };

        if is_sensitive_path(&search_path) {
            return Err(ToolError::ExecutionFailed(
                "Access denied: search path may contain credentials. \
                 Use `secret_list` and `secret_create` to manage credentials securely."
                    .to_string(),
            ));
        }

        let mut cmd = Command::new("rg");
        cmd.env_clear();
        for (key, val) in safe_env() {
            cmd.env(&key, &val);
        }
        for (key, val) in ctx.extra_env.as_ref() {
            cmd.env(key, val);
        }

        cmd.arg("--color").arg("never");
        cmd.arg("--no-heading");
        cmd.arg("--glob").arg("!.git");
        cmd.arg("--glob").arg("!node_modules");
        cmd.arg("--glob").arg("!target");

        match output_mode {
            "files_with_matches" => {
                cmd.arg("--files-with-matches");
            }
            "count" => {
                cmd.arg("--count");
            }
            "content" => {
                cmd.arg("-n");
            }
            _ => {
                return Err(ToolError::InvalidParameters(format!(
                    "Invalid output_mode '{}'. Must be: content, files_with_matches, or count",
                    output_mode
                )));
            }
        }

        if case_insensitive {
            cmd.arg("-i");
        }
        if let Some(g) = glob_filter {
            cmd.arg("--glob").arg(g);
        }

        cmd.arg("-e").arg(pattern);
        cmd.arg(&search_path);

        let output = tokio::time::timeout(Duration::from_secs(30), cmd.output())
            .await
            .map_err(|_| ToolError::Timeout(Duration::from_secs(30)))?
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ToolError::ExecutionFailed(
                        "ripgrep (rg) is not installed. Install it from: \
                         https://github.com/BurntSushi/ripgrep#installation"
                            .to_string(),
                    )
                } else {
                    ToolError::ExecutionFailed(format!("Failed to execute rg: {}", e))
                }
            })?;

        if output.status.code() == Some(2) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!(
                "ripgrep error: {}",
                stderr.trim()
            )));
        }

        let raw_output = String::from_utf8_lossy(&output.stdout);
        let truncated_output = if raw_output.len() > MAX_OUTPUT_SIZE {
            let mut end = MAX_OUTPUT_SIZE;
            while end > 0 && !raw_output.is_char_boundary(end) {
                end -= 1;
            }
            &raw_output[..end]
        } else {
            &raw_output
        };

        let lines: Vec<&str> = truncated_output.lines().collect();
        let effective_limit = match head_limit {
            Some(0) => lines.len(),
            Some(n) => n,
            None => DEFAULT_HEAD_LIMIT,
        };

        let paginated: Vec<&str> = lines.iter().take(effective_limit).copied().collect();
        let was_truncated = raw_output.len() > MAX_OUTPUT_SIZE || lines.len() > effective_limit;

        let result = match output_mode {
            "files_with_matches" => {
                let files: Vec<String> = paginated
                    .iter()
                    .map(|line| {
                        let path = line.trim().to_string();
                        std::path::Path::new(&path)
                            .strip_prefix(&search_path)
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or(path)
                    })
                    .filter(|p| !p.is_empty())
                    .collect();
                let count = files.len();
                serde_json::json!({
                    "files": files,
                    "count": count,
                    "truncated": was_truncated
                })
            }
            "count" => {
                let mut counts: Vec<serde_json::Value> = Vec::new();
                let mut total: u64 = 0;
                for line in &paginated {
                    if let Some((file, count_str)) = line.rsplit_once(':') {
                        let count = count_str.trim().parse::<u64>().unwrap_or(0);
                        let relative = std::path::Path::new(file)
                            .strip_prefix(&search_path)
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|_| file.to_string());
                        total += count;
                        counts.push(serde_json::json!({
                            "file": relative,
                            "count": count
                        }));
                    }
                }
                serde_json::json!({
                    "counts": counts,
                    "total": total,
                    "truncated": was_truncated
                })
            }
            _ => {
                let search_prefix = format!("{}/", search_path.display());
                let content: String = paginated
                    .iter()
                    .map(|line| line.strip_prefix(search_prefix.as_str()).unwrap_or(line))
                    .collect::<Vec<_>>()
                    .join("\n");
                serde_json::json!({
                    "content": content,
                    "truncated": was_truncated
                })
            }
        };

        Ok(ToolOutput::success(result, start.elapsed()))
    }

    fn requires_sanitization(&self) -> bool {
        true
    }

    fn requires_approval(&self, _params: &serde_json::Value) -> ApprovalRequirement {
        ApprovalRequirement::Never
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::Container
    }

    fn execution_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

    fn discovery_summary(&self) -> Option<ToolDiscoverySummary> {
        Some(ToolDiscoverySummary {
            notes: vec![
                "Default scope is 'workspace' — safe and sandboxed".into(),
                "Use scope='global' only when enabled in Settings -> Tools -> Local Search".into(),
                "Default output_mode is files_with_matches (paths only)".into(),
            ],
            examples: vec![
                serde_json::json!({"pattern": "fn main", "output_mode": "content"}),
                serde_json::json!({"pattern": "TODO", "glob": "*.rs"}),
                serde_json::json!({"pattern": "import", "output_mode": "count"}),
            ],
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn is_rg_available() -> bool {
        std::process::Command::new("rg")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    fn make_ctx() -> JobContext {
        JobContext::new("test", "test")
    }

    #[tokio::test]
    async fn test_global_scope_blocked_when_disabled() {
        let tool = LocalSearchTool::new().with_settings(LocalSearchSettings {
            allow_global_scope: false,
        });
        let ctx = make_ctx();
        let params = serde_json::json!({
            "pattern": "hello",
            "scope": "global"
        });
        let result = tool.execute(params, &ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Enable it in Settings"),
            "Expected 'Enable it in Settings' in error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_global_scope_allowed_when_enabled() {
        if !is_rg_available() {
            eprintln!("SKIPPING: ripgrep (rg) not installed");
            return;
        }
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "hello world\n").unwrap();

        let tool = LocalSearchTool::new()
            .with_settings(LocalSearchSettings {
                allow_global_scope: true,
            })
            .with_base_dir(dir.path().to_path_buf());

        let ctx = make_ctx();
        let params = serde_json::json!({
            "pattern": "hello",
            "scope": "global",
            "path": dir.path().to_str().unwrap()
        });
        let result = tool.execute(params, &ctx).await;
        assert!(
            result.is_ok(),
            "Expected success with allow_global_scope=true, got: {:?}",
            result.unwrap_err()
        );
    }

    #[tokio::test]
    async fn test_workspace_scope_does_not_require_setting() {
        if !is_rg_available() {
            eprintln!("SKIPPING: ripgrep (rg) not installed");
            return;
        }
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("src.rs"), "fn main() {}\n").unwrap();

        let tool = LocalSearchTool::new()
            .with_settings(LocalSearchSettings {
                allow_global_scope: false,
            })
            .with_base_dir(dir.path().to_path_buf());

        let ctx = make_ctx();
        let params = serde_json::json!({
            "pattern": "fn main",
            "scope": "workspace"
        });
        let result = tool.execute(params, &ctx).await;
        assert!(
            result.is_ok(),
            "Expected success for workspace scope, got: {:?}",
            result.unwrap_err()
        );
    }
}
