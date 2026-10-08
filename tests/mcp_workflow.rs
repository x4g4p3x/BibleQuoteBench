use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use biblequotebench::{
    BenchmarkCase, ReferenceRecord, TranslationCatalog,
    io::{read_json, read_jsonl},
    mcp::{McpConfig, McpServer},
};
use serde_json::{Value, json};
use tempfile::TempDir;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn config(temp: &TempDir) -> McpConfig {
    McpConfig {
        run_id: "test".into(),
        model: "fixture-assistant".into(),
        output_dir: temp.path().into(),
        case_limit: Some(2),
        translation: Some("bsb-2025-third-printing".into()),
        seed: "test-seed".into(),
    }
}

fn dataset() -> (TranslationCatalog, Vec<BenchmarkCase>, Vec<ReferenceRecord>) {
    (
        read_json(&root().join("data/dev/translations.json")).unwrap(),
        read_jsonl(&root().join("data/dev/cases.jsonl")).unwrap(),
        read_jsonl(&root().join("data/dev/references.jsonl")).unwrap(),
    )
}

fn server(config: &McpConfig) -> McpServer {
    let (catalog, cases, references) = dataset();
    McpServer::open(catalog, cases, references, config).unwrap()
}

fn initialize(version: &str) -> Value {
    json!({"jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
        "protocolVersion":version, "capabilities":{}, "clientInfo":{"name":"test", "version":"1"}
    }})
}

fn ready() -> Value {
    json!({"jsonrpc":"2.0", "method":"notifications/initialized"})
}

fn call(name: &str, arguments: Value) -> Value {
    let mut request =
        json!({"jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{"name":name}});
    request["params"]["arguments"] = arguments;
    request
}

fn exchange(server: &mut McpServer, requests: &[Value]) -> Vec<Value> {
    let input = requests
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let mut output = Vec::new();
    server.serve(input.as_bytes(), &mut output).unwrap();
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn tools(server: &mut McpServer, requests: &[Value]) -> Vec<Value> {
    let mut messages = vec![initialize("2025-11-25"), ready()];
    messages.extend_from_slice(requests);
    exchange(server, &messages).into_iter().skip(1).collect()
}

fn data(response: &Value) -> Value {
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn recall_answers_are_immutable_and_feedback_is_withheld() {
    let temp = TempDir::new().unwrap();
    let cfg = config(&temp);
    let mut server = server(&cfg);
    let issued = tools(
        &mut server,
        &[call("next_case", json!({})), call("next_case", json!({}))],
    );
    assert_eq!(issued[0], issued[1]);
    let case_id = data(&issued[0])["case_id"].clone();
    let (_, cases, references) = dataset();
    let expected = &references
        .iter()
        .find(|record| {
            record.translation
                == cases
                    .iter()
                    .find(|case| case.case_id == case_id)
                    .unwrap()
                    .translation
                && record.reference
                    == cases
                        .iter()
                        .find(|case| case.case_id == case_id)
                        .unwrap()
                        .reference
        })
        .unwrap()
        .text;
    assert!(!issued[0].to_string().contains(expected));
    let answer = json!({"case_id":case_id,"output":"  raw\nanswer  "});
    let results = tools(
        &mut server,
        &[
            call(
                "submit_answer",
                json!({"case_id":"not-issued","output":"out of order"}),
            ),
            call("finish_run", json!({})),
            call("submit_answer", answer.clone()),
            call("submit_answer", answer),
            call(
                "submit_answer",
                json!({"case_id":case_id,"output":"edited"}),
            ),
            call("benchmark_status", json!({})),
        ],
    );
    for index in [0, 1, 4] {
        assert_eq!(results[index]["result"]["isError"], true);
    }
    assert_eq!(data(&results[3])["already_recorded"], true);
    assert_eq!(data(&results[5])["answered_cases"], 1);
    assert!(
        !results
            .iter()
            .map(Value::to_string)
            .collect::<String>()
            .contains("exact_text")
    );
    assert!(!temp.path().join("run-test/scores.jsonl").exists());
    drop(server);
    let state: Value = read_json(&temp.path().join("run-test/session.json")).unwrap();
    assert_eq!(state["responses"][0]["output"], "  raw\nanswer  ");
    assert_eq!(state["identity"]["evidence"], "interactive_mcp");
}

#[test]
fn restarts_resume_pending_cases_and_export_existing_formats() {
    let temp = TempDir::new().unwrap();
    let cfg = config(&temp);
    let mut first = server(&cfg);
    let issued = tools(&mut first, &[call("next_case", json!({}))]);
    drop(first);
    let mut resumed = server(&cfg);
    let repeated = tools(&mut resumed, &[call("next_case", json!({}))]);
    assert_eq!(issued, repeated);
    let case_id = data(&issued[0])["case_id"].clone();
    tools(
        &mut resumed,
        &[call(
            "submit_answer",
            json!({"case_id":case_id,"output":""}),
        )],
    );
    let next = tools(&mut resumed, &[call("next_case", json!({}))]);
    let next_id = data(&next[0])["case_id"].clone();
    let (_, cases, references) = dataset();
    let selected = cases.iter().find(|case| case.case_id == next_id).unwrap();
    let expected = &references
        .iter()
        .find(|r| r.translation == selected.translation && r.reference == selected.reference)
        .unwrap()
        .text;
    let replies = tools(
        &mut resumed,
        &[
            call("next_case", json!({})),
            call(
                "submit_answer",
                json!({"case_id":next_id,"output":expected}),
            ),
            call("next_case", json!({})),
            call("finish_run", json!({})),
            call("finish_run", json!({})),
        ],
    );
    assert_eq!(data(&replies[2])["complete"], true);
    assert_eq!(data(&replies[3])["summary"]["exact_text_rate"], 0.5);
    assert_eq!(replies[3], replies[4]);
    assert!(!replies[3].to_string().contains(expected));
    drop(resumed);
    let mut finished = server(&cfg);
    assert_eq!(
        data(&tools(&mut finished, &[call("benchmark_status", json!({}))])[0])["finished"],
        true
    );
    let output = temp.path().join("run-test");
    let responses: Vec<biblequotebench::ResponseRecord> =
        read_jsonl(&output.join("responses.jsonl")).unwrap();
    let scores: Vec<biblequotebench::ScoreRecord> =
        read_jsonl(&output.join("scores.jsonl")).unwrap();
    assert_eq!(responses.len(), 2);
    assert!(responses.iter().all(|r| r.provider == "interactive_mcp"
        && r.execution.is_none()
        && r.resolved_model.is_none()));
    assert_eq!(
        scores[0].classification,
        biblequotebench::Classification::Empty
    );
    assert!(scores[1].exact_text);
    let trial: Value = read_json(&output.join("trial.json")).unwrap();
    assert_eq!(
        trial["responses_sha256"],
        biblequotebench::study::digest(&responses)
    );
    assert_eq!(trial["identity"]["model_identity"], "self_reported");
    assert!(
        fs::read_to_string(output.join("report.md"))
            .unwrap()
            .contains("not verified")
    );
}

#[test]
fn invalid_config_changed_inputs_corrupt_progress_and_concurrent_writers_are_rejected() {
    let temp = TempDir::new().unwrap();
    let base = config(&temp);
    for cfg in [
        McpConfig {
            run_id: "../escape".into(),
            ..base.clone()
        },
        McpConfig {
            run_id: String::new(),
            ..base.clone()
        },
        McpConfig {
            model: " ".into(),
            ..base.clone()
        },
        McpConfig {
            case_limit: Some(0),
            ..base.clone()
        },
        McpConfig {
            translation: Some("missing".into()),
            ..base.clone()
        },
    ] {
        let (catalog, cases, references) = dataset();
        assert!(McpServer::open(catalog, cases, references, &cfg).is_err());
    }
    let owner = server(&base);
    let (catalog, cases, references) = dataset();
    assert!(McpServer::open(catalog, cases, references, &base).is_err());
    drop(owner);
    for cfg in [
        McpConfig {
            model: "different".into(),
            ..base.clone()
        },
        McpConfig {
            case_limit: Some(1),
            ..base.clone()
        },
        McpConfig {
            seed: "changed-seed".into(),
            ..base.clone()
        },
    ] {
        let (catalog, cases, references) = dataset();
        assert!(McpServer::open(catalog, cases, references, &cfg).is_err());
    }
    let (catalog, cases, mut references) = dataset();
    references[0].text.push('!');
    assert!(McpServer::open(catalog, cases, references, &base).is_err());
    let path = temp.path().join("run-test/session.json");
    let saved: Value = read_json(&path).unwrap();
    for change in [json!({"finished":true}), json!({"responses":[{}]})] {
        let mut corrupt = saved.clone();
        for (name, value) in change.as_object().unwrap() {
            corrupt[name] = value.clone();
        }
        fs::write(&path, corrupt.to_string()).unwrap();
        let (catalog, cases, references) = dataset();
        assert!(McpServer::open(catalog, cases, references, &base).is_err());
    }
    let (catalog, mut cases, references) = dataset();
    cases[0].prompt_variant = biblequotebench::PromptVariant::CopyControl;
    assert!(McpServer::open(catalog, cases, references, &base).is_err());
}

#[test]
fn protocol_handshake_negotiation_errors_and_notifications_follow_mcp() {
    let temp = TempDir::new().unwrap();
    let mut server = server(&config(&temp));
    let replies = exchange(
        &mut server,
        &[
            call("next_case", json!({})),
            json!({"jsonrpc":"2.0", "id":1,"method":"initialize","params":{}}),
            initialize("future-version"),
            call("next_case", json!({})),
            ready(),
            json!({"jsonrpc":"2.0","id":"list","method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":4,"method":"ping"}),
            json!({"jsonrpc":"2.0","id":5,"method":"resources/list"}),
            json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"next_case","arguments":[]}}),
            call("unknown", json!({})),
            call("next_case", json!({"unexpected":true})),
            call(
                "submit_answer",
                json!({"case_id":"x","output":"x","extra":true}),
            ),
            json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"next_case","arguments":{}}}),
            call("benchmark_status", json!({})),
            json!({"id":1,"method":"ping"}),
            json!({"jsonrpc":"2.0","id":null,"method":"ping"}),
            json!([]),
        ],
    );
    assert_eq!(replies.len(), 15);
    assert_eq!(replies[0]["error"]["code"], -32000);
    assert_eq!(replies[1]["error"]["code"], -32602);
    assert_eq!(replies[2]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(replies[3]["error"]["code"], -32000);
    assert_eq!(replies[4]["id"], "list");
    assert_eq!(replies[4]["result"]["tools"].as_array().unwrap().len(), 6);
    assert_eq!(replies[5]["result"], json!({}));
    assert_eq!(replies[6]["error"]["code"], -32601);
    assert_eq!(replies[7]["error"]["code"], -32602);
    for index in [8, 9, 10] {
        assert_eq!(replies[index]["result"]["isError"], true);
    }
    assert!(data(&replies[11])["pending_case_id"].is_null());
    for reply in &replies[12..] {
        assert_eq!(reply["error"]["code"], -32600);
    }
    let mut bytes = Vec::new();
    server.serve(&b"invalid JSON\n"[..], &mut bytes).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap()["error"]["code"],
        -32700
    );
    for version in ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"] {
        let replies = exchange(
            &mut server,
            &[
                initialize(version),
                ready(),
                call("benchmark_status", json!({})),
            ],
        );
        assert_eq!(replies[0]["result"]["protocolVersion"], version);
        assert_eq!(
            replies[1]["result"].get("structuredContent").is_some(),
            version >= "2025-06-18"
        );
    }
    let too_large = vec![b'x'; 1_048_577];
    assert!(server.serve(too_large.as_slice(), Vec::new()).is_err());
}

#[test]
fn translation_selection_submission_limits_and_no_issued_case_are_enforced() {
    let temp = TempDir::new().unwrap();
    let mut cfg = config(&temp);
    cfg.translation = Some("asv-1901".into());
    cfg.case_limit = Some(1);
    let mut server = server(&cfg);
    let first = tools(&mut server, &[call("next_case", json!({}))]);
    let case_id = data(&first[0])["case_id"].clone();
    let replies = tools(
        &mut server,
        &[
            call(
                "submit_answer",
                json!({"case_id":"not-issued","output":"too early"}),
            ),
            call("next_case", json!({})),
            call(
                "submit_answer",
                json!({"case_id":case_id,"output":"x".repeat(16385)}),
            ),
            call(
                "submit_answer",
                json!({"case_id":case_id,"output":"unknown"}),
            ),
            call("finish_run", json!({})),
            call("next_case", json!({})),
        ],
    );
    assert_eq!(replies[0]["result"]["isError"], true);
    assert_eq!(data(&replies[1])["case_id"], case_id);
    assert_eq!(replies[2]["result"]["isError"], true);
    assert_eq!(data(&replies[5])["complete"], true);
}

fn idle_config(temp: &TempDir) -> McpConfig {
    McpConfig {
        run_id: String::new(),
        model: String::new(),
        ..config(temp)
    }
}

#[test]
fn idle_server_runs_multiple_trials_and_resumes_original_settings() {
    let temp = TempDir::new().unwrap();
    let mut server = server(&idle_config(&temp));
    let initial = tools(
        &mut server,
        &[
            call("benchmark_status", json!({})),
            call("next_case", json!({})),
        ],
    );
    assert_eq!(data(&initial[0])["active_trial"], false);
    assert_eq!(initial[1]["result"]["isError"], true);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    let begin =
        json!({"run_id":"first","model":"selected-model-1","case_limit":1,"seed":"trial-seed"});
    let started = tools(
        &mut server,
        &[
            call("begin_trial", begin.clone()),
            call("begin_trial", begin),
            call("resume_trial", json!({"run_id":"first"})),
        ],
    );
    assert_eq!(started[0], started[1]);
    assert_eq!(started[0], started[2]);
    assert_eq!(data(&started[0])["selection"]["seed"], "trial-seed");
    let issued = tools(&mut server, &[call("next_case", json!({}))]);
    let blocked = tools(
        &mut server,
        &[
            call(
                "begin_trial",
                json!({"run_id":"second","model":"selected-model-2"}),
            ),
            call("resume_trial", json!({"run_id":"missing"})),
            call("begin_trial", json!({"run_id":"first","model":"changed"})),
        ],
    );
    assert!(
        blocked
            .iter()
            .all(|reply| reply["result"]["isError"] == true)
    );
    drop(server);
    let mut defaults = idle_config(&temp);
    defaults.seed = "other-default".into();
    let mut resumed = self::server(&defaults);
    let status = tools(
        &mut resumed,
        &[
            call("resume_trial", json!({"run_id":"first"})),
            call("next_case", json!({})),
        ],
    );
    assert_eq!(data(&status[0])["model"], "selected-model-1");
    assert_eq!(data(&status[0])["total_cases"], 1);
    assert_eq!(issued[0], status[1]);
    let completed = tools(
        &mut resumed,
        &[
            call(
                "submit_answer",
                json!({"case_id":data(&issued[0])["case_id"],"output":""}),
            ),
            call("finish_run", json!({})),
            call(
                "begin_trial",
                json!({"run_id":"second","model":"selected-model-2","case_limit":1}),
            ),
        ],
    );
    assert!(
        completed
            .iter()
            .all(|reply| reply["result"]["isError"] == false)
    );
    assert_eq!(data(&completed[2])["answered_cases"], 0);
    assert_eq!(data(&completed[2])["model"], "selected-model-2");
    let repeated = tools(
        &mut resumed,
        &[
            call(
                "begin_trial",
                json!({"run_id":"first","model":"selected-model-1"}),
            ),
            call("resume_trial", json!({"run_id":"first"})),
        ],
    );
    assert_eq!(repeated[0]["result"]["isError"], true);
    assert_eq!(data(&repeated[1])["finished"], true);
    assert!(temp.path().join("run-second/session.json").exists());
}

#[test]
fn lifecycle_rejects_unsafe_labels_missing_runs_and_failed_switches_without_losing_active_trial() {
    let temp = TempDir::new().unwrap();
    let cfg = idle_config(&temp);
    let mut manager = server(&cfg);
    for arguments in [
        json!({"run_id":"../escape","model":"model"}),
        json!({"run_id":"test","model":" "}),
        json!({"run_id":"test","model":"client (model unspecified)"}),
        json!({"run_id":"test","model":"model","seed":" "}),
        json!({"run_id":"test","model":"model","extra":true}),
    ] {
        assert_eq!(
            tools(&mut manager, &[call("begin_trial", arguments)])[0]["result"]["isError"],
            true
        );
    }
    assert_eq!(
        tools(
            &mut manager,
            &[call("resume_trial", json!({"run_id":"../escape"}))]
        )[0]["result"]["isError"],
        true
    );
    tools(
        &mut manager,
        &[call(
            "begin_trial",
            json!({"run_id":"active","model":"original"}),
        )],
    );
    let owner = self::server(&McpConfig {
        run_id: "locked".into(),
        model: "other".into(),
        ..config(&temp)
    });
    let failures = tools(
        &mut manager,
        &[
            call("resume_trial", json!({"run_id":"locked"})),
            call("resume_trial", json!({"run_id":"missing"})),
            call("benchmark_status", json!({})),
        ],
    );
    assert_eq!(failures[0]["result"]["isError"], true);
    assert_eq!(failures[1]["result"]["isError"], true);
    assert_eq!(data(&failures[2])["run_id"], "active");
    drop(owner);
}

#[test]
fn schema_one_checkpoints_keep_original_prefix_prompts_and_pending_progress() {
    let temp = TempDir::new().unwrap();
    let cfg = config(&temp);
    drop(server(&cfg));
    let (catalog, cases, _) = dataset();
    let original: Vec<_> = cases
        .into_iter()
        .filter(|case| Some(&case.translation) == cfg.translation.as_ref())
        .take(2)
        .collect();
    let path = temp.path().join("run-test/session.json");
    let mut saved: Value = read_json(&path).unwrap();
    let identity = saved["identity"].as_object_mut().unwrap();
    identity.remove("selection");
    identity.insert("schema_version".into(), json!(1));
    identity.insert(
        "cases_sha256".into(),
        json!(biblequotebench::study::digest(&original)),
    );
    identity.insert(
        "expected_case_ids".into(),
        json!(
            original
                .iter()
                .map(|case| &case.case_id)
                .collect::<Vec<_>>()
        ),
    );
    let prompts: Vec<_> = original
        .iter()
        .map(|case| {
            biblequotebench::render_prompt(
                case,
                catalog
                    .translations
                    .iter()
                    .find(|spec| spec.id == case.translation)
                    .unwrap(),
            )
        })
        .collect();
    identity.insert(
        "prompts_sha256".into(),
        json!(biblequotebench::study::digest(&prompts)),
    );
    saved["pending"] = json!(true);
    fs::write(&path, saved.to_string()).unwrap();
    let mut manager = server(&idle_config(&temp));
    let replies = tools(
        &mut manager,
        &[
            call("resume_trial", json!({"run_id":"test"})),
            call("next_case", json!({})),
        ],
    );
    assert_eq!(data(&replies[1])["case_id"], original[0].case_id);
    assert_eq!(data(&replies[1])["prompt"], prompts[0]);
    assert!(data(&replies[0])["selection"].is_null());
    let after: Value = read_json(&path).unwrap();
    assert_eq!(after, saved);
}

#[test]
fn resume_rejects_semantically_invalid_saved_responses_without_rewriting_them() {
    let temp = TempDir::new().unwrap();
    let cfg = config(&temp);
    let mut original = server(&cfg);
    let issued = tools(&mut original, &[call("next_case", json!({}))]);
    tools(
        &mut original,
        &[call(
            "submit_answer",
            json!({"case_id":data(&issued[0])["case_id"],"output":"raw answer"}),
        )],
    );
    drop(original);
    let path = temp.path().join("run-test/session.json");
    let saved: Value = read_json(&path).unwrap();
    let mut manager = server(&idle_config(&temp));
    for (field, value) in [
        ("case_id", json!("unknown")),
        ("run_id", json!("other")),
        ("model", json!("other")),
        ("provider", json!("openai")),
        ("resolved_model", json!("model")),
        ("error", json!("failed")),
        ("temperature", json!(0)),
        ("reasoning_effort", json!("high")),
        ("seed", json!(1)),
        ("provider_request_id", json!("request")),
        ("system_fingerprint", json!("fingerprint")),
        (
            "execution",
            json!({"input_tokens":1,"output_tokens":2,"stop_reason":null,"truncated":false}),
        ),
        ("output", json!("x".repeat(16385))),
    ] {
        let mut corrupt = saved.clone();
        corrupt["responses"][0][field] = value;
        fs::write(&path, corrupt.to_string()).unwrap();
        let result = tools(
            &mut manager,
            &[call("resume_trial", json!({"run_id":"test"}))],
        );
        assert_eq!(result[0]["result"]["isError"], true, "{field}");
        assert!(
            result[0]["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("saved MCP response does not match")
        );
        assert_eq!(read_json::<Value>(&path).unwrap(), corrupt);
    }
    fs::write(&path, saved.to_string()).unwrap();
    let result = tools(
        &mut manager,
        &[call("resume_trial", json!({"run_id":"test"}))],
    );
    assert_eq!(data(&result[0])["answered_cases"], 1);
}

#[test]
fn unsupported_selection_and_unspecified_model_cannot_issue_prompts() {
    let temp = TempDir::new().unwrap();
    let cfg = McpConfig {
        model: "client (model unspecified)".into(),
        ..config(&temp)
    };
    let mut original = server(&cfg);
    let result = tools(
        &mut original,
        &[
            call("benchmark_status", json!({})),
            call("next_case", json!({})),
        ],
    );
    assert_eq!(data(&result[0])["needs_model_label"], true);
    assert_eq!(result[1]["result"]["isError"], true);
    drop(original);
    let path = temp.path().join("run-test/session.json");
    let mut saved: Value = read_json(&path).unwrap();
    assert_eq!(saved["pending"], false);
    saved["identity"]["selection"]["method"] = json!("unsupported");
    fs::write(&path, saved.to_string()).unwrap();
    let mut manager = server(&idle_config(&temp));
    let result = tools(
        &mut manager,
        &[call("resume_trial", json!({"run_id":"test"}))],
    );
    assert_eq!(result[0]["result"]["isError"], true);
    assert!(
        result[0]["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("unsupported selection method")
    );
    assert_eq!(read_json::<Value>(&path).unwrap(), saved);
}

struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Client {
    fn request(&mut self, value: &Value) -> Value {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{value}").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        assert!(self.stdout.read_line(&mut line).unwrap() > 0);
        serde_json::from_str(&line).unwrap()
    }
}

#[test]
fn real_stdio_process_exports_scores_without_provider_configuration() {
    let temp = TempDir::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_biblequotebench"))
        .current_dir(temp.path())
        .args([
            "mcp",
            "--run-id",
            "process",
            "--model",
            "fixture",
            "--case-limit",
            "1",
            "--translation",
            "bsb-2025-third-printing",
            "--output-dir",
        ])
        .arg(temp.path())
        .arg("--translations")
        .arg(root().join("data/dev/translations.json"))
        .arg("--cases")
        .arg(root().join("data/dev/cases.jsonl"))
        .arg("--references")
        .arg(root().join("data/dev/references.jsonl"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let mut client = Client {
        child,
        stdin: Some(stdin),
        stdout,
    };
    assert_eq!(
        client.request(&initialize("2025-11-25"))["result"]["serverInfo"]["name"],
        "biblequotebench"
    );
    writeln!(client.stdin.as_mut().unwrap(), "{}", ready()).unwrap();
    let issued = data(&client.request(&call("next_case", json!({}))));
    assert_eq!(
        client.request(&call(
            "submit_answer",
            json!({"case_id":issued["case_id"],"output":""})
        ))["result"]["isError"],
        false
    );
    let finished = data(&client.request(&call("finish_run", json!({}))));
    assert_eq!(finished["summary"]["responses"], 1);
    assert_eq!(finished["progress"]["provider_api_calls"], 0);
    assert!(temp.path().join("run-process/trial.json").exists());
    drop(client.stdin.take());
    assert!(
        client.child.wait().unwrap().success(),
        "server must exit cleanly on stdin EOF"
    );
}
