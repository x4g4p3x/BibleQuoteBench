//! Subscription-backed recall with an authoritative, tool-free Responses gate.
//! The trusted Codex executable supplies authentication, never the model context.

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use fs2::FileExt;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::{domain::ExecutionMetadata, io::write_json};

const UPSTREAM: &str = "https://chatgpt.com/backend-api/codex/responses";
const MAX_REQUEST: u64 = 2 * 1024 * 1024;
const MAX_RESPONSE: u64 = 8 * 1024 * 1024;
const INSTRUCTIONS: &str = "Answer the Bible quotation prompt from internal recall only. No browsing, retrieval, tools, or external sources are available. Output only the passage text, exactly as recalled, without commentary.";
const DISABLED_FEATURES: &[&str] = &[
    "shell_tool",
    "apps",
    "remote_plugin",
    "memories",
    "hooks",
    "multi_agent",
    "multi_agent_v2",
    "browser_use",
    "computer_use",
    "image_generation",
    "code_mode_host",
    "sleep_tool",
    "workspace_dependencies",
    "skill_search",
    "enable_request_compression",
];

/// Supported fixed reasoning settings for subscription recall.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    #[default]
    Low,
    High,
    Xhigh,
    Max,
}

impl ReasoningEffort {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }
}

/// Operator-selected subscription runner. MCP callers cannot change these settings.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub program: PathBuf,
    pub model: String,
    pub reasoning_effort: ReasoningEffort,
    pub timeout_seconds: u64,
}

/// Resume-bound description of the executable and gate contract, without credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerIdentity {
    pub method: String,
    pub model: String,
    /// Missing in legacy identities, which always used low effort.
    #[serde(default)]
    pub reasoning_effort: ReasoningEffort,
    pub cli_version: String,
    pub executable_sha256: String,
    pub instructions_sha256: String,
    pub timeout_seconds: u64,
}

/// Prepared local runner; credentials stay in private temporary storage.
#[derive(Clone)]
pub struct RestrictedRunner {
    program: PathBuf,
    auth_file: PathBuf,
    identity: RunnerIdentity,
    upstream: String,
}

/// Only accepted after the entire upstream response passes the gate.
#[derive(Debug)]
pub struct RecallResult {
    pub text: String,
    pub resolved_model: String,
    pub execution: ExecutionMetadata,
    pub request_sha256: String,
    pub response_sha256: String,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

impl RestrictedRunner {
    /// Checks the installed CLI and subscription credentials without making a model call.
    ///
    /// # Errors
    /// Rejects unsupported executables, unsafe model labels, and API-key authentication.
    pub fn prepare(config: &RunnerConfig) -> Result<Self> {
        ensure!(
            !config.model.is_empty()
                && config.model.len() <= 256
                && config
                    .model
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')),
            "restricted model must be an explicit model identifier"
        );
        ensure!(
            (1..=300).contains(&config.timeout_seconds),
            "timeout must be 1-300 seconds"
        );
        let program = resolve_program(&config.program)?;
        let version = Command::new(&program).arg("--version").output()?;
        ensure!(
            version.status.success(),
            "cannot determine Codex CLI version"
        );
        let version = String::from_utf8(version.stdout)?.trim().to_owned();
        ensure!(
            version.starts_with("codex-cli "),
            "restricted runner requires Codex CLI"
        );
        let help = Command::new(&program).args(["exec", "--help"]).output()?;
        let help = String::from_utf8(help.stdout)?;
        for flag in ["--ignore-user-config", "--ephemeral", "--strict-config"] {
            ensure!(
                help.contains(flag),
                "Codex CLI lacks required isolation option {flag}; update Codex"
            );
        }
        let home = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .map(|home| PathBuf::from(home).join(".codex"))
            })
            .context("cannot locate Codex subscription credentials")?;
        let auth_file = home.join("auth.json");
        subscription_auth(&auth_file)?;
        Ok(Self {
            identity: RunnerIdentity {
                method: "codex_responses_gate_v1".into(),
                model: config.model.clone(),
                reasoning_effort: config.reasoning_effort,
                cli_version: version,
                executable_sha256: hash(&fs::read(&program)?),
                instructions_sha256: hash(INSTRUCTIONS.as_bytes()),
                timeout_seconds: config.timeout_seconds,
            },
            program,
            auth_file,
            upstream: UPSTREAM.into(),
        })
    }

    pub fn identity(&self) -> &RunnerIdentity {
        &self.identity
    }

    /// Runs exactly one request; never retries or falls back to API billing.
    ///
    /// # Errors
    /// Rejects tool output, malformed/incomplete responses, timeouts, and changed binaries.
    pub fn recall(&self, prompt: &str, audit_dir: &Path) -> Result<RecallResult> {
        ensure!(
            hash(&fs::read(&self.program)?) == self.identity.executable_sha256,
            "Codex executable changed; start a new trial"
        );
        let (scratch, home, workspace) = private_workspace()?;
        let auth = subscription_auth(&self.auth_file)?;
        private_auth(&home.join("auth.json"), &auth)?;
        let server = Server::http("127.0.0.1:0")
            .map_err(|_| anyhow::anyhow!("cannot bind local recall gate"))?;
        let address = server
            .server_addr()
            .to_ip()
            .context("gate requires a loopback address")?;
        let route = format!(
            "/{}/responses",
            scratch
                .path()
                .file_name()
                .context("temporary directory name missing")?
                .to_string_lossy()
        );
        let base = format!("http://{address}{}", route.trim_end_matches("/responses"));
        let mut command = Command::new(&self.program);
        command.env_clear();
        for key in [
            "PATH",
            "SystemRoot",
            "WINDIR",
            "COMSPEC",
            "PATHEXT",
            "TEMP",
            "TMP",
            "USERPROFILE",
            "HOME",
            "APPDATA",
            "LOCALAPPDATA",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
            .env("CODEX_HOME", &home)
            .current_dir(&workspace)
            .args([
                "exec",
                "--ignore-user-config",
                "--ephemeral",
                "--strict-config",
                "--skip-git-repo-check",
                "--json",
                "--color",
                "never",
                "-m",
                &self.identity.model,
                "-c",
                "model_provider=\"bqb_gate\"",
                "-c",
                "forced_login_method=\"chatgpt\"",
                "-c",
                "cli_auth_credentials_store=\"file\"",
                "-c",
                "web_search=\"disabled\"",
                "-c",
                "project_doc_max_bytes=0",
            ]);
        command.args([
            "-c",
            &format!(
                "model_reasoning_effort={:?}",
                self.identity.reasoning_effort.as_str()
            ),
        ]);
        for feature in DISABLED_FEATURES {
            command.args(["-c", &format!("features.{feature}=false")]);
        }
        command.args(["-c", &format!("model_providers.bqb_gate={{name=\"BibleQuoteBench recall gate\",base_url={base:?},wire_api=\"responses\",requires_openai_auth=true,supports_websockets=false,request_max_retries=0,stream_max_retries=0}}"), "-"])
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
        let mut child = ChildGuard(
            command
                .spawn()
                .context("cannot launch restricted Codex runner")?,
        );
        child
            .0
            .stdin
            .take()
            .context("missing runner stdin")?
            .write_all(b"Complete the recall request supplied by the benchmark gate.\n")?;
        let result = self.drive(
            &server,
            &route,
            &mut child,
            prompt,
            audit_dir,
            &self.upstream,
        );
        drop(child);
        persist_auth_refresh(&self.auth_file, &home.join("auth.json"), &auth)?;
        result
    }

    fn drive(
        &self,
        server: &Server,
        route: &str,
        child: &mut ChildGuard,
        prompt: &str,
        audit: &Path,
        upstream: &str,
    ) -> Result<RecallResult> {
        let deadline = Instant::now() + Duration::from_secs(self.identity.timeout_seconds);
        let mut result = None;
        let mut catalogue_seen = false;
        loop {
            ensure!(
                Instant::now() < deadline,
                "restricted recall timed out; do not automatically retry this case"
            );
            if let Some(request) = server.recv_timeout(Duration::from_millis(100))? {
                if request.method() == &Method::Get
                    && request.url().split('?').next()
                        == Some(&format!("{}/models", route.trim_end_matches("/responses")))
                {
                    ensure!(
                        !catalogue_seen && result.is_none(),
                        "unexpected repeated model catalogue request"
                    );
                    catalogue_seen = true;
                    request.respond(
                        Response::from_string("{\"models\":[]}").with_header(
                            Header::from_bytes("Content-Type", "application/json")
                                .expect("constant header"),
                        ),
                    )?;
                    continue;
                }
                ensure!(
                    result.is_none(),
                    "runner attempted more than one model request; trial blocked"
                );
                let remaining = deadline.saturating_duration_since(Instant::now());
                ensure!(!remaining.is_zero(), "restricted recall timed out");
                let client = Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .no_proxy()
                    .timeout(remaining)
                    .build()?;
                result = Some(forward(
                    request,
                    &client,
                    route,
                    upstream,
                    prompt,
                    &self.identity.model,
                    self.identity.reasoning_effort,
                    audit,
                )?);
            }
            if let Some(status) = child.0.try_wait()? {
                ensure!(
                    status.success(),
                    "restricted Codex runner failed; check subscription login and client version"
                );
                return result.context("runner exited without passing through the recall gate");
            }
        }
    }
}

fn private_workspace() -> Result<(tempfile::TempDir, PathBuf, PathBuf)> {
    let scratch = tempfile::Builder::new()
        .prefix("bqb-restricted-")
        .tempdir()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700))?;
    }
    let home = scratch.path().join("home");
    let workspace = scratch.path().join("workspace");
    fs::create_dir(&home)?;
    fs::create_dir(&workspace)?;
    Ok((scratch, home, workspace))
}

fn resolve_program(program: &Path) -> Result<PathBuf> {
    if program.components().count() > 1 || program.is_absolute() {
        return fs::canonicalize(program).context("Codex executable not found");
    }
    for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        let candidate = directory.join(program);
        if candidate.is_file() {
            return Ok(fs::canonicalize(candidate)?);
        }
        if cfg!(windows) {
            let candidate = candidate.with_extension("exe");
            if candidate.is_file() {
                return Ok(fs::canonicalize(candidate)?);
            }
        }
    }
    bail!("Codex executable not found; install Codex CLI or supply --codex-bin")
}

fn subscription_auth(path: &Path) -> Result<Value> {
    let bytes = fs::read(path)
        .context("file-backed Codex login unavailable; sign in with ChatGPT using Codex CLI")?;
    parse_subscription_auth(&bytes)
}

fn parse_subscription_auth(bytes: &[u8]) -> Result<Value> {
    let mut auth: Value = serde_json::from_slice(bytes)
        .map_err(|_| anyhow::anyhow!("invalid Codex credential file"))?;
    ensure!(
        auth.get("auth_mode").is_none_or(|mode| mode == "chatgpt")
            && auth["OPENAI_API_KEY"].is_null()
            && auth["tokens"]["access_token"]
                .as_str()
                .is_some_and(|token| !token.is_empty())
            && auth["tokens"]["refresh_token"]
                .as_str()
                .is_some_and(|token| !token.is_empty()),
        "restricted trials require ChatGPT subscription login; API-key authentication is rejected"
    );
    auth["OPENAI_API_KEY"] = Value::Null;
    Ok(auth)
}

fn private_auth(path: &Path, auth: &Value) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    serde_json::to_writer(&mut file, auth)?;
    Ok(())
}

// Preserve native OAuth refreshes without replacing a concurrent account change.
fn persist_auth_refresh(source: &Path, isolated: &Path, original: &Value) -> Result<()> {
    let refreshed = subscription_auth(isolated)?;
    if &refreshed == original {
        return Ok(());
    }
    ensure!(
        refreshed["tokens"]["account_id"] == original["tokens"]["account_id"],
        "isolated subscription account changed; trial blocked"
    );
    let lock = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(source.with_extension("bqb-refresh.lock"))?;
    lock.try_lock_exclusive()
        .context("subscription credentials busy; refreshed trial blocked")?;
    ensure!(
        &subscription_auth(source)? == original,
        "subscription credentials changed concurrently; trial blocked"
    );
    let mut replacement =
        tempfile::NamedTempFile::new_in(source.parent().context("credential directory missing")?)?;
    replacement
        .as_file()
        .set_permissions(fs::metadata(source)?.permissions())?;
    serde_json::to_writer(replacement.as_file_mut(), &refreshed)?;
    replacement.as_file_mut().sync_all()?;
    replacement.persist(source).map_err(|_| {
        anyhow::anyhow!("cannot retain refreshed subscription credentials; sign in again")
    })?;
    Ok(())
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn canonical_request(prompt: &str, model: &str, effort: ReasoningEffort) -> Value {
    json!({"model":model, "stream":true, "store":false, "instructions":INSTRUCTIONS,
        "input":[{"role":"user","content":[{"type":"input_text","text":prompt}]}],
        "tools":[], "tool_choice":"none", "parallel_tool_calls":false,
        "reasoning":{"effort":effort.as_str(),"summary":"auto"}, "include":["reasoning.encrypted_content"]})
}

#[allow(clippy::too_many_arguments)]
fn forward(
    mut request: Request,
    client: &Client,
    route: &str,
    upstream: &str,
    prompt: &str,
    model: &str,
    effort: ReasoningEffort,
    audit: &Path,
) -> Result<RecallResult> {
    let result = gated_response(
        &mut request,
        client,
        route,
        upstream,
        prompt,
        model,
        effort,
        audit,
    );
    match result {
        Ok((bytes, recall)) => {
            let response = Response::from_data(bytes).with_header(
                Header::from_bytes("Content-Type", "text/event-stream").expect("constant header"),
            );
            request.respond(response)?;
            Ok(recall)
        }
        Err(error) => {
            let _ = request.respond(
                Response::from_string("Restricted recall gate rejected the request or response")
                    .with_status_code(StatusCode(502)),
            );
            Err(error)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn gated_response(
    request: &mut Request,
    client: &Client,
    route: &str,
    upstream: &str,
    prompt: &str,
    model: &str,
    effort: ReasoningEffort,
    audit: &Path,
) -> Result<(Vec<u8>, RecallResult)> {
    ensure!(
        request.method() == &Method::Post && request.url() == route,
        "unexpected gate route or method: {} {}",
        request.method(),
        request
            .url()
            .split('?')
            .next()
            .unwrap_or("")
            .rsplit('/')
            .next()
            .unwrap_or("")
    );
    ensure!(
        !request
            .headers()
            .iter()
            .any(|h| h.field.equiv("Content-Encoding")),
        "compressed requests are not accepted"
    );
    let mut bytes = Vec::new();
    request
        .as_reader()
        .take(MAX_REQUEST + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        u64::try_from(bytes.len())? <= MAX_REQUEST,
        "gate request exceeds size limit"
    );
    let offered: Value = serde_json::from_slice(&bytes).context("invalid gate request JSON")?;
    ensure!(
        offered["model"] == model && offered["stream"] == true,
        "runner changed the requested model or transport"
    );
    let canonical = canonical_request(prompt, model, effort);
    fs::create_dir_all(audit)?;
    write_json(&audit.join("request.json"), &canonical)?;
    let mut upstream_request = client
        .post(upstream)
        .json(&canonical)
        .header("Accept", "text/event-stream");
    let mut authenticated = false;
    for header in request.headers() {
        let name = header.field.as_str().as_str().to_ascii_lowercase();
        if [
            "authorization",
            "chatgpt-account-id",
            "openai-beta",
            "originator",
            "version",
        ]
        .contains(&name.as_str())
        {
            if name == "authorization" {
                authenticated = true;
            }
            upstream_request = upstream_request.header(name, header.value.as_str());
        }
    }
    ensure!(
        authenticated,
        "runner did not supply subscription authentication"
    );
    let response = upstream_request.send().map_err(|_| {
        anyhow::anyhow!("subscription request failed; billing may be uncertain; no retry")
    })?;
    ensure!(
        response.status().is_success(),
        "subscription service rejected request (HTTP {}); no retry",
        response.status().as_u16()
    );
    let mut bytes = Vec::new();
    response
        .take(MAX_RESPONSE + 1)
        .read_to_end(&mut bytes)
        .context("incomplete subscription response; no retry")?;
    ensure!(
        u64::try_from(bytes.len())? <= MAX_RESPONSE,
        "subscription response exceeds size limit"
    );
    fs::write(audit.join("response.sse"), &bytes)?;
    let mut recall = validate_response(&bytes, model)?;
    recall.request_sha256 = hash(&serde_json::to_vec(&canonical)?);
    recall.response_sha256 = hash(&bytes);
    Ok((bytes, recall))
}

fn validate_item(item: &Value) -> Result<()> {
    match item["type"].as_str() {
        Some("reasoning") => Ok(()),
        Some("message") => {
            ensure!(item["role"] == "assistant", "unexpected response role");
            for part in item["content"]
                .as_array()
                .context("invalid response content")?
            {
                ensure!(
                    matches!(part["type"].as_str(), Some("output_text" | "refusal")),
                    "non-text response content rejected"
                );
            }
            Ok(())
        }
        _ => bail!("tool use or unknown output item rejected before client execution"),
    }
}

pub(crate) fn read_audit(
    audit: &Path,
    prompt: &str,
    model: &str,
    effort: ReasoningEffort,
) -> Result<RecallResult> {
    let canonical = canonical_request(prompt, model, effort);
    let saved: Value = crate::io::read_json(&audit.join("request.json"))?;
    ensure!(saved == canonical, "restricted gate request audit changed");
    let bytes = fs::read(audit.join("response.sse"))?;
    ensure!(
        u64::try_from(bytes.len())? <= MAX_RESPONSE,
        "gate audit exceeds response size limit"
    );
    let mut recall = validate_response(&bytes, model)?;
    recall.request_sha256 = hash(&serde_json::to_vec(&canonical)?);
    recall.response_sha256 = hash(&bytes);
    Ok(recall)
}

fn validate_response(bytes: &[u8], requested_model: &str) -> Result<RecallResult> {
    let text = std::str::from_utf8(bytes).context("response is not UTF-8")?;
    let mut completed = None;
    for block in text.replace("\r\n", "\n").split("\n\n") {
        let data = block
            .lines()
            .filter_map(|line| {
                line.strip_prefix("data: ")
                    .or_else(|| line.strip_prefix("data:"))
            })
            .collect::<Vec<_>>()
            .join("\n");
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let event: Value = serde_json::from_str(&data).context("invalid subscription event")?;
        let kind = event["type"]
            .as_str()
            .context("missing subscription event type")?;
        ensure!(
            event_allowed(kind),
            "tool use, failure, or unknown stream event rejected before client execution"
        );
        if let Some(item) = event.get("item") {
            validate_item(item)?;
        }
        if let Some(part) = event.get("part") {
            ensure!(
                matches!(
                    part["type"].as_str(),
                    Some("output_text" | "refusal" | "summary_text" | "reasoning_text")
                ),
                "unknown stream content rejected"
            );
        }
        if let Some(response) = event.get("response") {
            for item in response["output"]
                .as_array()
                .context("missing response output")?
            {
                validate_item(item)?;
            }
        }
        if kind == "response.completed" {
            ensure!(completed.is_none(), "duplicate completion rejected");
            completed = Some(event["response"].clone());
        }
    }
    let response = completed.context("subscription response did not complete; no retry")?;
    ensure!(
        response["status"] == "completed" && response["model"] == requested_model,
        "response status or model does not match the trial"
    );
    let mut text = String::new();
    let mut refusal = false;
    for item in response["output"]
        .as_array()
        .context("invalid completed output")?
    {
        if item["type"] == "message" {
            for part in item["content"]
                .as_array()
                .context("invalid completed content")?
            {
                let field = if part["type"] == "refusal" {
                    refusal = true;
                    "refusal"
                } else {
                    "text"
                };
                text.push_str(part[field].as_str().context("missing response text")?);
            }
        }
    }
    ensure!(
        text.chars().count() <= 16_384,
        "answer exceeds 16384 characters"
    );
    Ok(RecallResult {
        text,
        resolved_model: requested_model.into(),
        request_sha256: String::new(),
        response_sha256: String::new(),
        execution: ExecutionMetadata {
            input_tokens: Some(
                response["usage"]["input_tokens"]
                    .as_u64()
                    .context("missing input usage")?,
            ),
            output_tokens: Some(
                response["usage"]["output_tokens"]
                    .as_u64()
                    .context("missing output usage")?,
            ),
            stop_reason: Some("completed".into()),
            refusal,
            ..ExecutionMetadata::default()
        },
    })
}

fn event_allowed(kind: &str) -> bool {
    matches!(
        kind,
        "response.created"
            | "response.in_progress"
            | "response.completed"
            | "response.output_item.added"
            | "response.output_item.done"
            | "response.content_part.added"
            | "response.content_part.done"
            | "response.output_text.delta"
            | "response.output_text.done"
            | "response.refusal.delta"
            | "response.refusal.done"
            | "response.reasoning_summary_part.added"
            | "response.reasoning_summary_part.done"
            | "response.reasoning_summary_text.delta"
            | "response.reasoning_summary_text.done"
            | "response.reasoning_text.delta"
            | "response.reasoning_text.done"
    )
}

#[cfg(test)]
pub(crate) mod tests;
