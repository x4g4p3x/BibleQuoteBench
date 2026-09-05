//! Closed-book HTTP adapters for supported model providers.

use std::{env, time::Duration};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::{
    BenchmarkCase, ReferenceRecord, ResponseRecord, TranslationCatalog, prompt::execution_prompt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProviderKind {
    Openai,
    Anthropic,
    Gemini,
    Xai,
    OpenaiCompatible,
}

impl ProviderKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Xai => "xai",
            Self::OpenaiCompatible => "openai_compatible",
        }
    }

    pub const fn default_api_key_env(self) -> Option<&'static str> {
        match self {
            Self::Openai => Some("OPENAI_API_KEY"),
            Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            Self::Gemini => Some("GEMINI_API_KEY"),
            Self::Xai => Some("XAI_API_KEY"),
            Self::OpenaiCompatible => None,
        }
    }

    pub const fn default_base_url(self) -> &'static str {
        match self {
            Self::Openai => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Self::Xai => "https://api.x.ai/v1",
            Self::OpenaiCompatible => "http://localhost:11434/v1",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub model: String,
    pub run_id: String,
    pub api_key_env: Option<String>,
    pub base_url: Option<String>,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub max_output_tokens: u32,
    pub case_limit: Option<usize>,
    pub fail_fast: bool,
}

#[derive(Debug)]
struct Completion {
    text: String,
    request_id: Option<String>,
    resolved_model: Option<String>,
    system_fingerprint: Option<String>,
    execution: crate::domain::ExecutionMetadata,
}

/// Validates execution settings without accessing credentials or the network.
///
/// # Errors
/// Rejects empty identities and unsupported parameter combinations.
pub fn validate_config(config: &ProviderConfig) -> Result<()> {
    if config.run_id.trim().is_empty() || config.model.trim().is_empty() {
        bail!("run_id and model must not be empty");
    }
    if config.reasoning_effort.is_some()
        && !matches!(
            config.kind,
            ProviderKind::Openai | ProviderKind::Xai | ProviderKind::Anthropic
        )
    {
        bail!("explicit reasoning effort is supported only by Responses and Anthropic adapters");
    }
    if config.max_output_tokens == 0 {
        bail!("max_output_tokens must be positive");
    }
    if config.kind == ProviderKind::Anthropic {
        if let Some(effort) = &config.reasoning_effort {
            if !["low", "medium", "high", "xhigh", "max"].contains(&effort.as_str()) {
                bail!("unsupported Claude effort: {effort}");
            }
        }
        if config.model.starts_with("claude-fable-5")
            && config
                .temperature
                .is_some_and(|t| (t - 1.0).abs() > f32::EPSILON)
        {
            bail!("Fable requires temperature 1.0 or an omitted temperature");
        }
    }
    Ok(())
}

/// Executes closed-book prompts against one provider, retaining per-case errors.
///
/// No tools or retrieval facilities are offered in any request.
///
/// # Errors
///
/// Returns an error for invalid configuration or, when `fail_fast` is enabled,
/// the first provider failure. Otherwise failures are represented in the returned
/// response records.
pub fn run_cases(
    config: &ProviderConfig,
    cases: &[BenchmarkCase],
    catalog: &TranslationCatalog,
) -> Result<Vec<ResponseRecord>> {
    run_cases_with_supplied(config, cases, catalog, &[])
}

/// Runs a diagnostic dataset, supplying corpus text exclusively to copy controls.
///
/// # Errors
/// Returns configuration, missing-copy-text, or fail-fast provider errors.
pub fn run_cases_with_supplied(
    config: &ProviderConfig,
    cases: &[BenchmarkCase],
    catalog: &TranslationCatalog,
    supplied: &[ReferenceRecord],
) -> Result<Vec<ResponseRecord>> {
    validate_config(config)?;
    let key_name = config
        .api_key_env
        .as_deref()
        .or_else(|| config.kind.default_api_key_env());
    let api_key = key_name
        .map(|name| {
            env::var(name).with_context(|| format!("environment variable {name} is not set"))
        })
        .transpose()?;
    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or_else(|| config.kind.default_base_url())
        .trim_end_matches('/');
    let client = Client::builder()
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("BibleQuoteBench/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let limit = config.case_limit.unwrap_or(cases.len()).min(cases.len());
    let mut records = Vec::with_capacity(limit);

    for case in cases.iter().take(limit) {
        let translation = catalog
            .translations
            .iter()
            .find(|translation| translation.id == case.translation)
            .with_context(|| format!("case {} uses unknown translation", case.case_id))?;
        let text = supplied
            .iter()
            .find(|record| {
                record.translation == case.translation && record.reference == case.reference
            })
            .map(|record| record.text.as_str());
        let prompt = execution_prompt(case, translation, text)?;
        let result = complete(&client, config, base_url, api_key.as_deref(), &prompt);
        match result {
            Ok(completion) => records.push(ResponseRecord {
                case_id: case.case_id.clone(),
                run_id: config.run_id.clone(),
                provider: config.kind.name().to_owned(),
                model: config.model.clone(),
                resolved_model: completion.resolved_model,
                error: if completion.text.is_empty() && !completion.execution.truncated {
                    Some("provider returned no visible text".into())
                } else {
                    None
                },
                output: completion.text,
                temperature: config.temperature,
                reasoning_effort: config.reasoning_effort.clone(),
                seed: None,
                provider_request_id: completion.request_id,
                system_fingerprint: completion.system_fingerprint,
                execution: Some(completion.execution),
            }),
            Err(error) if config.fail_fast => {
                return Err(error).with_context(|| case.case_id.clone());
            }
            Err(error) => records.push(ResponseRecord {
                case_id: case.case_id.clone(),
                run_id: config.run_id.clone(),
                provider: config.kind.name().to_owned(),
                model: config.model.clone(),
                resolved_model: None,
                output: String::new(),
                error: Some(truncate_error(&format!("{error:#}"))),
                temperature: config.temperature,
                reasoning_effort: config.reasoning_effort.clone(),
                seed: None,
                provider_request_id: None,
                system_fingerprint: None,
                execution: None,
            }),
        }
    }
    Ok(records)
}

fn complete(
    client: &Client,
    config: &ProviderConfig,
    base_url: &str,
    api_key: Option<&str>,
    prompt: &str,
) -> Result<Completion> {
    match config.kind {
        ProviderKind::Openai | ProviderKind::Xai => {
            complete_responses(client, config, base_url, required_key(api_key)?, prompt)
        }
        ProviderKind::Anthropic => {
            complete_anthropic(client, config, base_url, required_key(api_key)?, prompt)
        }
        ProviderKind::Gemini => {
            complete_gemini(client, config, base_url, required_key(api_key)?, prompt)
        }
        ProviderKind::OpenaiCompatible => {
            complete_chat_completions(client, config, base_url, api_key, prompt)
        }
    }
}

fn complete_responses(
    client: &Client,
    config: &ProviderConfig,
    base_url: &str,
    api_key: &str,
    prompt: &str,
) -> Result<Completion> {
    let mut body = json!({
        "model": config.model,
        "input": prompt,
        "max_output_tokens": config.max_output_tokens,
        "store": false,
        "tools": [],
        "tool_choice": "none"
    });
    insert_temperature(&mut body, config.temperature);
    if let Some(effort) = &config.reasoning_effort {
        body["reasoning"] = json!({"effort": effort});
    }
    let value = post_json(
        client,
        &format!("{base_url}/responses"),
        &[("Authorization", format!("Bearer {api_key}"))],
        &body,
    )?;
    let text = extract_responses_text(&value).unwrap_or_default();
    Ok(completion_metadata(&value, text))
}

fn extract_responses_text(value: &Value) -> Result<String> {
    let text = value
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("content").and_then(Value::as_array))
        .flatten()
        .filter(|part| part.get("type").and_then(Value::as_str) == Some("output_text"))
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    if text.is_empty() {
        bail!("provider response contained no output_text");
    }
    Ok(text)
}

fn complete_anthropic(
    client: &Client,
    config: &ProviderConfig,
    base_url: &str,
    api_key: &str,
    prompt: &str,
) -> Result<Completion> {
    let mut body = json!({
        "model": config.model,
        "max_tokens": config.max_output_tokens,
        "messages": [{"role": "user", "content": prompt}]
    });
    insert_temperature(&mut body, config.temperature);
    if let Some(effort) = &config.reasoning_effort {
        body["output_config"] = json!({"effort": effort});
    }
    let value = post_json(
        client,
        &format!("{base_url}/messages"),
        &[
            ("x-api-key", api_key.to_owned()),
            ("anthropic-version", "2023-06-01".to_owned()),
        ],
        &body,
    )?;
    let text = extract_anthropic_text(&value).unwrap_or_default();
    Ok(completion_metadata(&value, text))
}

fn extract_anthropic_text(value: &Value) -> Result<String> {
    let text = value
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    if text.is_empty() {
        bail!("provider response contained no text block");
    }
    Ok(text)
}

fn complete_gemini(
    client: &Client,
    config: &ProviderConfig,
    base_url: &str,
    api_key: &str,
    prompt: &str,
) -> Result<Completion> {
    let model = config
        .model
        .strip_prefix("models/")
        .unwrap_or(&config.model);
    if !model
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        bail!("Gemini model contains unsupported URL characters");
    }
    let mut generation = json!({"maxOutputTokens": config.max_output_tokens});
    insert_temperature(&mut generation, config.temperature);
    let body = json!({
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": generation
    });
    let value = post_json(
        client,
        &format!("{base_url}/models/{model}:generateContent"),
        &[("x-goog-api-key", api_key.to_owned())],
        &body,
    )?;
    let text = extract_gemini_text(&value).unwrap_or_default();
    Ok(Completion {
        text,
        request_id: value
            .get("responseId")
            .and_then(Value::as_str)
            .map(str::to_owned),
        resolved_model: value
            .get("modelVersion")
            .and_then(Value::as_str)
            .map(str::to_owned),
        system_fingerprint: None,
        execution: execution_metadata(&value),
    })
}

fn extract_gemini_text(value: &Value) -> Result<String> {
    let text = value
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    if text.is_empty() {
        bail!("provider response contained no candidate text");
    }
    Ok(text)
}

fn complete_chat_completions(
    client: &Client,
    config: &ProviderConfig,
    base_url: &str,
    api_key: Option<&str>,
    prompt: &str,
) -> Result<Completion> {
    let mut body = json!({
        "model": config.model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": config.max_output_tokens
    });
    insert_temperature(&mut body, config.temperature);
    let headers = api_key
        .map(|key| vec![("Authorization", format!("Bearer {key}"))])
        .unwrap_or_default();
    let value = post_json(
        client,
        &format!("{base_url}/chat/completions"),
        &headers,
        &body,
    )?;
    let text = value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .context("provider response contained no choices[0].message.content")?
        .to_owned();
    Ok(completion_metadata(&value, text))
}

fn post_json(
    client: &Client,
    url: &str,
    headers: &[(&str, String)],
    body: &Value,
) -> Result<Value> {
    // One attempt only: transport failures may already have incurred a charge.
    let mut request = client.post(url).json(body);
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    let response = request.send().with_context(|| format!("POST {url}"))?;
    let status = response.status();
    let text = response.text().context("reading provider response")?;
    if !status.is_success() {
        bail!("provider returned HTTP {status}: {}", truncate_error(&text));
    }
    serde_json::from_str(&text).context("parsing provider JSON response")
}

fn execution_metadata(value: &Value) -> crate::domain::ExecutionMetadata {
    let number = |path| value.pointer(path).and_then(Value::as_u64);
    let input = number("/usage/input_tokens")
        .map(|n| {
            n.saturating_add(number("/usage/cache_read_input_tokens").unwrap_or(0))
                .saturating_add(number("/usage/cache_creation_input_tokens").unwrap_or(0))
        })
        .or_else(|| number("/usage/prompt_tokens"))
        .or_else(|| number("/usageMetadata/promptTokenCount"));
    let output = number("/usage/output_tokens")
        .or_else(|| number("/usage/completion_tokens"))
        .or_else(|| {
            number("/usageMetadata/candidatesTokenCount")
                .map(|n| n.saturating_add(number("/usageMetadata/thoughtsTokenCount").unwrap_or(0)))
        });
    let reason = [
        "/incomplete_details/reason",
        "/stop_reason",
        "/choices/0/finish_reason",
        "/candidates/0/finishReason",
        "/status",
    ]
    .iter()
    .find_map(|path| value.pointer(path).and_then(Value::as_str))
    .map(str::to_owned);
    let truncated = reason
        .as_deref()
        .is_some_and(|r| ["max_tokens", "max_output_tokens", "length", "MAX_TOKENS"].contains(&r));
    crate::domain::ExecutionMetadata {
        input_tokens: input,
        output_tokens: output,
        stop_reason: reason,
        truncated,
        ..Default::default()
    }
}

fn completion_metadata(value: &Value, text: String) -> Completion {
    Completion {
        execution: execution_metadata(value),
        text,
        request_id: value.get("id").and_then(Value::as_str).map(str::to_owned),
        resolved_model: value
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_owned),
        system_fingerprint: value
            .get("system_fingerprint")
            .and_then(Value::as_str)
            .map(str::to_owned),
    }
}

fn insert_temperature(body: &mut Value, temperature: Option<f32>) {
    if let Some(temperature) = temperature {
        body.as_object_mut()
            .expect("request body is an object")
            .insert("temperature".to_owned(), json!(temperature));
    }
}

fn required_key(key: Option<&str>) -> Result<&str> {
    key.context("provider requires an API key environment variable")
}

fn truncate_error(error: &str) -> String {
    error.chars().take(1_000).collect()
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc::{self, Receiver},
        thread::JoinHandle,
    };

    use crate::{BibleReference, CaseStratum, LicenseKind, PromptVariant, TranslationSpec};

    use super::*;

    fn fixture_config(kind: ProviderKind, base_url: String) -> ProviderConfig {
        ProviderConfig {
            kind,
            model: "fixture-model".to_owned(),
            run_id: "fixture-run".to_owned(),
            api_key_env: None,
            base_url: Some(base_url),
            temperature: Some(0.0),
            reasoning_effort: None,
            max_output_tokens: 64,
            case_limit: None,
            fail_fast: true,
        }
    }

    fn mock_server(body: &'static str) -> (String, Receiver<String>, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            sender.send(String::from_utf8(request).unwrap()).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        (format!("http://{address}"), receiver, handle)
    }

    fn client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
    }

    #[test]
    fn openai_metadata_is_extracted() {
        let value = json!({"id":"resp_1","model":"pinned-model","system_fingerprint":"fp_1"});
        let completion = completion_metadata(&value, "text".to_owned());
        assert_eq!(completion.request_id.as_deref(), Some("resp_1"));
        assert_eq!(completion.resolved_model.as_deref(), Some("pinned-model"));
        assert_eq!(completion.system_fingerprint.as_deref(), Some("fp_1"));
    }

    #[test]
    fn provider_defaults_are_closed_book_endpoints() {
        assert_eq!(
            ProviderKind::Openai.default_base_url(),
            "https://api.openai.com/v1"
        );
        assert_eq!(ProviderKind::Xai.default_api_key_env(), Some("XAI_API_KEY"));
    }

    #[test]
    fn responses_text_is_collected_across_typed_output_items() {
        let value = json!({"output":[
            {"type":"reasoning","content":[]},
            {"type":"message","content":[
                {"type":"output_text","text":"First"},
                {"type":"refusal","refusal":"no"}
            ]},
            {"type":"message","content":[{"type":"output_text","text":" second"}]}
        ]});
        assert_eq!(extract_responses_text(&value).unwrap(), "First second");
    }

    #[test]
    fn anthropic_text_blocks_are_collected() {
        let value = json!({"content":[
            {"type":"text","text":"First"},
            {"type":"thinking","thinking":"hidden"},
            {"type":"text","text":" second"}
        ]});
        assert_eq!(extract_anthropic_text(&value).unwrap(), "First second");
    }

    #[test]
    fn gemini_parts_are_collected() {
        let value = json!({"candidates":[{"content":{"parts":[
            {"text":"First"}, {"text":" second"}
        ]}}]});
        assert_eq!(extract_gemini_text(&value).unwrap(), "First second");
    }

    #[test]
    fn responses_adapter_sends_a_closed_book_request() {
        let (base_url, request, server) = mock_server(
            r#"{"id":"resp_1","model":"resolved","output":[{"content":[{"type":"output_text","text":"answer"}]}]}"#,
        );
        let config = fixture_config(ProviderKind::Openai, base_url.clone());
        let completion =
            complete_responses(&client(), &config, &base_url, "test-key", "prompt").unwrap();
        assert_eq!(completion.text, "answer");
        let request = request.recv().unwrap();
        assert!(request.starts_with("POST /responses HTTP/1.1"));
        let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body["store"], false);
        assert_eq!(body["tools"], json!([]));
        assert_eq!(body["tool_choice"], "none");
        server.join().unwrap();
    }

    #[test]
    fn anthropic_and_gemini_adapters_follow_their_wire_contracts() {
        let (base_url, request, server) = mock_server(
            r#"{"id":"msg_1","model":"resolved","content":[{"type":"text","text":"answer"}]}"#,
        );
        let config = fixture_config(ProviderKind::Anthropic, base_url.clone());
        let completion =
            complete_anthropic(&client(), &config, &base_url, "test-key", "prompt").unwrap();
        assert_eq!(completion.text, "answer");
        let request = request.recv().unwrap().to_ascii_lowercase();
        assert!(request.starts_with("post /messages http/1.1"));
        assert!(request.contains("x-api-key: test-key"));
        assert!(request.contains("anthropic-version: 2023-06-01"));
        server.join().unwrap();

        let (base_url, request, server) = mock_server(
            r#"{"responseId":"gem_1","modelVersion":"resolved","candidates":[{"content":{"parts":[{"text":"answer"}]}}]}"#,
        );
        let config = fixture_config(ProviderKind::Gemini, base_url.clone());
        let completion =
            complete_gemini(&client(), &config, &base_url, "test-key", "prompt").unwrap();
        assert_eq!(completion.text, "answer");
        let request = request.recv().unwrap().to_ascii_lowercase();
        assert!(request.starts_with("post /models/fixture-model:generatecontent http/1.1"));
        assert!(request.contains("x-goog-api-key: test-key"));
        server.join().unwrap();
    }

    #[test]
    fn claude_effort_and_thinking_only_cutoff_keep_usage() {
        let (base_url, request, server) = mock_server(
            r#"{"model":"claude-fable-5-1","stop_reason":"max_tokens","content":[{"type":"thinking","thinking":""}],"usage":{"input_tokens":80,"output_tokens":4096}}"#,
        );
        let mut config = fixture_config(ProviderKind::Anthropic, base_url.clone());
        config.model = "claude-fable-5-1".into();
        config.reasoning_effort = Some("max".into());
        config.temperature = None;
        validate_config(&config).unwrap();
        let result =
            complete_anthropic(&client(), &config, &base_url, "test-key", "prompt").unwrap();
        assert!(result.text.is_empty());
        assert!(result.execution.truncated);
        assert_eq!(result.execution.output_tokens, Some(4096));
        let request = request.recv().unwrap();
        let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body["output_config"]["effort"], "max");
        assert!(body.get("temperature").is_none());
        server.join().unwrap();
        config.reasoning_effort = Some("ultra".into());
        assert!(validate_config(&config).is_err());
        config.reasoning_effort = Some("high".into());
        config.temperature = Some(0.0);
        assert!(validate_config(&config).is_err());
        config.temperature = Some(1.0);
        validate_config(&config).unwrap();
    }

    #[test]
    fn all_adapters_capture_token_limits_and_billable_usage_without_double_counting_thinking() {
        for value in [
            json!({"status":"incomplete","incomplete_details":{"reason":"max_output_tokens"},"usage":{"input_tokens":80,"output_tokens":100,"output_tokens_details":{"reasoning_tokens":90}}}),
            json!({"choices":[{"finish_reason":"length"}],"usage":{"prompt_tokens":80,"completion_tokens":100}}),
            json!({"candidates":[{"finishReason":"MAX_TOKENS"}],"usageMetadata":{"promptTokenCount":80,"candidatesTokenCount":10,"thoughtsTokenCount":90}}),
            json!({"stop_reason":"max_tokens","usage":{"input_tokens":50,"cache_read_input_tokens":20,"cache_creation_input_tokens":10,"output_tokens":100}}),
        ] {
            let metadata = execution_metadata(&value);
            assert!(metadata.truncated);
            assert_eq!(metadata.input_tokens, Some(80));
            assert_eq!(metadata.output_tokens, Some(100));
        }
        assert_eq!(execution_metadata(&json!({})).input_tokens, None);
    }

    #[test]
    fn compatible_adapter_runs_a_complete_case() {
        let (base_url, request, server) = mock_server(
            r#"{"id":"chat_1","model":"resolved","choices":[{"message":{"content":"answer"}}]}"#,
        );
        let case = BenchmarkCase {
            case_id: "BQ-FIXTURE".to_owned(),
            translation: "fixture".to_owned(),
            reference: BibleReference {
                book: "John".to_owned(),
                chapter: 3,
                verse_start: 16,
                verse_end: None,
            },
            stratum: CaseStratum::ExtremelyFamous,
            prompt_variant: PromptVariant::Canonical,
        };
        let catalog = TranslationCatalog {
            schema_version: 1,
            translations: vec![TranslationSpec {
                id: "fixture".to_owned(),
                name: "Fixture Bible".to_owned(),
                abbreviation: "FIX".to_owned(),
                edition: "1".to_owned(),
                license_kind: LicenseKind::PublicDomain,
                license_url: "https://example.test/license".to_owned(),
                source_url: "https://example.test/source".to_owned(),
                redistribute_reference_text: true,
            }],
        };
        let records = run_cases(
            &fixture_config(ProviderKind::OpenaiCompatible, base_url),
            &[case],
            &catalog,
        )
        .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].output, "answer");
        assert_eq!(records[0].resolved_model.as_deref(), Some("resolved"));
        assert!(
            request
                .recv()
                .unwrap()
                .starts_with("POST /chat/completions HTTP/1.1")
        );
        server.join().unwrap();
    }
}
