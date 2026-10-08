//! Local, tool-only MCP stdio server for auditable interactive recall trials.
//!
//! This transport intentionally never calls a provider or samples a client model.
//! Reference text and scoring feedback remain private until all answers are locked.

mod session;

use std::io::{BufRead, Write};

use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

pub use session::{McpConfig, McpServer};

const PROTOCOL_VERSIONS: &[&str] = &["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];
const MAX_MESSAGE_BYTES: u64 = 1_048_576;

#[derive(Default, PartialEq, Eq)]
enum Lifecycle {
    #[default]
    New,
    Initializing,
    Ready,
}

#[derive(Deserialize)]
struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    capabilities: serde_json::Map<String, Value>,
    #[serde(rename = "clientInfo")]
    client_info: ClientInfo,
}

#[derive(Deserialize)]
struct ClientInfo {
    name: String,
    version: String,
}

#[derive(Deserialize)]
struct CallParams {
    name: String,
    #[serde(default = "empty_object")]
    arguments: Value,
}

fn empty_object() -> Value {
    json!({})
}

impl McpServer {
    /// Serves newline-delimited JSON-RPC until the client closes stdin.
    ///
    /// # Errors
    /// Returns transport failures or rejects messages exceeding the 1 MiB limit.
    /// Diagnostics belong on stderr; stdout contains only MCP messages.
    pub fn serve(&mut self, mut reader: impl BufRead, mut writer: impl Write) -> Result<()> {
        let mut lifecycle = Lifecycle::New;
        let mut protocol_version = String::new();
        loop {
            let mut line = Vec::new();
            let length = std::io::Read::take(&mut reader, MAX_MESSAGE_BYTES + 1)
                .read_until(b'\n', &mut line)?;
            if length == 0 {
                return Ok(());
            }
            if u64::try_from(length)? > MAX_MESSAGE_BYTES {
                bail!("MCP message exceeds 1 MiB; connection closed");
            }
            let response = match serde_json::from_slice::<Value>(&line) {
                Ok(request) => self.dispatch(&request, &mut lifecycle, &mut protocol_version),
                Err(_) => Some(rpc_error(&Value::Null, -32700, "Parse error")),
            };
            if let Some(response) = response {
                serde_json::to_writer(&mut writer, &response)?;
                writer.write_all(b"\n")?;
                writer.flush()?;
            }
        }
    }

    fn dispatch(
        &mut self,
        request: &Value,
        lifecycle: &mut Lifecycle,
        protocol_version: &mut String,
    ) -> Option<Value> {
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str);
        if request.get("jsonrpc") != Some(&json!("2.0"))
            || method.is_none()
            || id
                .as_ref()
                .is_some_and(|id| !id.is_string() && !id.is_i64() && !id.is_u64())
        {
            return Some(rpc_error(&Value::Null, -32600, "Invalid Request"));
        }
        let method = method.expect("checked above");
        let Some(id) = id else {
            if method == "notifications/initialized" && *lifecycle == Lifecycle::Initializing {
                *lifecycle = Lifecycle::Ready;
            }
            // Notifications must never invoke a state-changing tool.
            return None;
        };
        let params = request.get("params").cloned().unwrap_or_else(empty_object);
        let result = match method {
            "initialize" if *lifecycle == Lifecycle::New => {
                match serde_json::from_value::<InitializeParams>(params) {
                    Ok(params) => {
                        let _ = (
                            params.capabilities,
                            params.client_info.name,
                            params.client_info.version,
                        );
                        *protocol_version =
                            if PROTOCOL_VERSIONS.contains(&params.protocol_version.as_str()) {
                                params.protocol_version
                            } else {
                                "2025-11-25".into()
                            };
                        *lifecycle = Lifecycle::Initializing;
                        Ok(json!({
                            "protocolVersion": protocol_version,
                            "capabilities": {"tools": {"listChanged": false}},
                            "serverInfo": {"name": "biblequotebench", "version": env!("CARGO_PKG_VERSION")},
                            "instructions": "Interactive recall trial. Call benchmark_status, next_case, submit_answer, then finish_run. Answer from recall; do not browse, retrieve Bible text, or inspect local reference files. Submit the exact passage only. Scores are withheld until every case has an immutable answer. Model identity is self-reported; this is not controlled closed-book evidence."
                        }))
                    }
                    Err(_) => Err((-32602, "Invalid initialize parameters")),
                }
            }
            "ping" => Ok(json!({})),
            _ if *lifecycle != Lifecycle::Ready => {
                Err((-32000, "Complete the MCP initialization handshake first"))
            }
            "tools/list" => Ok(json!({"tools": tool_definitions()})),
            "tools/call" => match serde_json::from_value::<CallParams>(params) {
                Ok(params) if params.arguments.is_object() => {
                    let result = self.call_tool(&params.name, params.arguments);
                    match result {
                        Ok(value) => {
                            let mut result = json!({"content": [{"type": "text", "text": value.to_string()}], "isError": false});
                            if ["2025-06-18", "2025-11-25"].contains(&protocol_version.as_str()) {
                                result["structuredContent"] = value;
                            }
                            Ok(result)
                        }
                        Err(error) => Ok(
                            json!({"content": [{"type": "text", "text": error.to_string()}], "isError": true}),
                        ),
                    }
                }
                _ => Err((-32602, "Invalid tools/call parameters")),
            },
            _ => Err((-32601, "Method not found")),
        };
        Some(match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err((code, message)) => rpc_error(&id, code, message),
        })
    }
}

fn rpc_error(id: &Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn tool_definitions() -> Value {
    let empty = json!({"type": "object", "properties": {}, "additionalProperties": false});
    json!([
        {
            "name": "benchmark_status",
            "description": "Read run identity and progress without answers or scoring feedback.",
            "inputSchema": empty,
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "next_case",
            "description": "Get the next recall prompt, with no reference answer. Repeated calls return the same case until its answer is submitted.",
            "inputSchema": empty,
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
        },
        {
            "name": "submit_answer",
            "description": "Lock the exact raw passage for the currently issued case. Empty answers are valid. Answers cannot be changed. Returns progress only, never correctness feedback.",
            "inputSchema": {"type": "object", "properties": {
                "case_id": {"type": "string"},
                "output": {"type": "string", "maxLength": 16384}
            }, "required": ["case_id", "output"], "additionalProperties": false},
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
        },
        {
            "name": "finish_run",
            "description": "After every case is answered, score the immutable run and save responses.jsonl, scores.jsonl and reports. Returns aggregate metrics only, never reference text. Safe to repeat.",
            "inputSchema": empty,
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
        }
    ])
}
