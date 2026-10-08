use std::{fs, path::Path};

use serde_json::{Value, json};
use tempfile::TempDir;

use super::{McpConfig, McpServer, tool_definitions};
use crate::{
    io::{read_json, read_jsonl, write_json},
    restricted::{
        RestrictedRunner,
        tests::{fake_program, fake_runner, stream, upstream},
    },
};

fn config(directory: &Path, model: &str) -> McpConfig {
    McpConfig {
        run_id: "restricted-test".into(),
        model: model.into(),
        output_dir: directory.join("results"),
        case_limit: Some(2),
        translation: Some("bsb-2025-third-printing".into()),
        seed: "fixture-seed".into(),
    }
}

fn open(config: &McpConfig, runner: Option<RestrictedRunner>) -> anyhow::Result<McpServer> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/dev");
    let catalog = read_json(&root.join("translations.json"))?;
    let cases = read_jsonl(&root.join("cases.jsonl"))?;
    let references = read_jsonl(&root.join("references.jsonl"))?;
    if let Some(runner) = runner {
        McpServer::open_restricted(catalog, cases, references, config, runner)
    } else {
        McpServer::open(catalog, cases, references, config)
    }
}

#[test]
fn restricted_protocol_advertises_runner_answers_and_no_manual_submission() {
    let names: Vec<_> = tool_definitions(true)
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_owned())
        .collect();
    assert!(names.iter().any(|name| name == "answer_case"));
    assert!(!names.iter().any(|name| name == "submit_answer"));
    let temp = TempDir::new().unwrap();
    let program = fake_program(temp.path());
    let runner = fake_runner(temp.path(), &program, "fixture", "http://127.0.0.1:9");
    let mut config = config(temp.path(), "fixture");
    config.run_id.clear();
    config.model.clear();
    let mut server = open(&config, Some(runner)).unwrap();
    assert_eq!(
        server.call_tool("benchmark_status", json!({})).unwrap()["evidence"],
        "restricted_codex"
    );
    assert!(
        server
            .call_tool("begin_trial", json!({"run_id":"new","model":"other"}))
            .is_err()
    );
    assert!(
        server
            .call_tool(
                "begin_trial",
                json!({"run_id":"new","model":"fixture","case_limit":3})
            )
            .is_err()
    );
    server
        .call_tool("begin_trial", json!({"run_id":"new","model":"fixture"}))
        .unwrap();
    let next = server.call_tool("next_case", json!({})).unwrap();
    assert!(next.get("prompt").is_none());
    assert!(
        server
            .call_tool(
                "submit_answer",
                json!({"case_id":next["case_id"],"output":"copied answer"})
            )
            .is_err()
    );
    assert!(
        server
            .call_tool(
                "answer_case",
                json!({"case_id":next["case_id"],"output":"copied answer"})
            )
            .is_err()
    );
    assert!(
        server
            .call_tool("answer_case", json!({"case_id":"wrong"}))
            .is_err()
    );
    let input = format!(
        "{}\n{}\n{}\n",
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})
    );
    let mut output = Vec::new();
    server.serve(input.as_bytes(), &mut output).unwrap();
    let messages: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        messages[0]["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("answer_case")
    );
    assert_eq!(messages[1]["result"]["tools"], tool_definitions(true));
}

#[test]
fn restricted_answers_are_audited_immutable_and_resumed_without_reexecution() {
    let temp = TempDir::new().unwrap();
    let program = fake_program(temp.path());
    let (url, worker) = upstream(
        "fixture",
        vec![
            stream(
                "fixture",
                json!([{"type":"output_text","text":"  original\nanswer  "}]),
            ),
            stream("fixture", json!([{"type":"refusal","refusal":""}])),
        ],
    );
    let runner = fake_runner(temp.path(), &program, "fixture", &url);
    let config = config(temp.path(), "fixture");
    let mut server = open(&config, Some(runner.clone())).unwrap();
    let first = server.call_tool("next_case", json!({})).unwrap();
    let accepted = server
        .call_tool("answer_case", json!({"case_id":first["case_id"]}))
        .unwrap();
    assert_eq!(accepted["progress"]["answered_cases"], 1);
    assert!(accepted.get("summary").is_none());
    assert_eq!(
        server
            .call_tool("answer_case", json!({"case_id":first["case_id"]}))
            .unwrap()["already_recorded"],
        true
    );
    drop(server);
    let mut server = open(&config, Some(runner.clone())).unwrap();
    let second = server.call_tool("next_case", json!({})).unwrap();
    server
        .call_tool("answer_case", json!({"case_id":second["case_id"]}))
        .unwrap();
    let finished = server.complete_restricted().unwrap();
    assert_eq!(finished["progress"]["finished"], true);
    assert_eq!(finished["summary"]["refusal_rate"], 0.5);
    assert_eq!(worker.join().unwrap().len(), 2);
    let directory = config.output_dir.join("run-restricted-test");
    let trial: Value = read_json(&directory.join("trial.json")).unwrap();
    assert_eq!(trial["identity"]["schema_version"], 3);
    assert_eq!(trial["identity"]["evidence"], "restricted_codex");
    let original = fs::read(directory.join("session.json")).unwrap();
    drop(server);
    assert!(open(&config, None).is_err());
    let saved: Value = serde_json::from_slice(&original).unwrap();
    for (pointer, value) in [
        ("/responses/0/output", json!("modified answer")),
        ("/responses/0/provider", json!("interactive_mcp")),
        ("/restricted_attempts/0/state", json!("blocked")),
        ("/restricted_attempts/0/request_sha256", json!("wrong")),
        ("/restricted_attempts/0/case_id", json!("wrong")),
        ("/restricted_attempts", json!([])),
    ] {
        let mut changed = saved.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        write_json(&directory.join("session.json"), &changed).unwrap();
        let before = fs::read(directory.join("session.json")).unwrap();
        assert!(open(&config, Some(runner.clone())).is_err());
        assert_eq!(fs::read(directory.join("session.json")).unwrap(), before);
    }
    fs::write(directory.join("session.json"), original).unwrap();
    let mut server = open(&config, Some(runner)).unwrap();
    assert_eq!(
        server.complete_restricted().unwrap()["progress"]["finished"],
        true
    );
}

#[test]
fn blocked_and_interrupted_attempts_cannot_be_replayed_or_promoted_from_interactive() {
    let temp = TempDir::new().unwrap();
    let program = fake_program(temp.path());
    let runner = fake_runner(temp.path(), &program, "fail", "http://127.0.0.1:9");
    let config = config(temp.path(), "fail");
    let mut server = open(&config, Some(runner.clone())).unwrap();
    let issued = server.call_tool("next_case", json!({})).unwrap();
    assert!(
        server
            .call_tool("answer_case", json!({"case_id":issued["case_id"]}))
            .is_err()
    );
    assert_eq!(
        server.call_tool("benchmark_status", json!({})).unwrap()["blocked"],
        true
    );
    let checkpoint = config.output_dir.join("run-restricted-test/session.json");
    let original = fs::read(&checkpoint).unwrap();
    drop(server);
    for state in ["blocked", "started"] {
        let mut saved: Value = serde_json::from_slice(&original).unwrap();
        saved["restricted_attempts"][0]["state"] = json!(state);
        write_json(&checkpoint, &saved).unwrap();
        let before = fs::read(&checkpoint).unwrap();
        let mut server = open(&config, Some(runner.clone())).unwrap();
        assert!(
            server
                .call_tool("answer_case", json!({"case_id":issued["case_id"]}))
                .unwrap_err()
                .to_string()
                .contains("already has")
        );
        assert!(server.complete_restricted().is_err());
        assert_eq!(fs::read(&checkpoint).unwrap(), before);
    }
    let interactive = TempDir::new().unwrap();
    let mut legacy = config.clone();
    legacy.output_dir = interactive.path().into();
    let mut server = open(&legacy, None).unwrap();
    assert!(server.complete_restricted().is_err());
    assert!(
        server
            .call_tool("answer_case", json!({"case_id":"any"}))
            .is_err()
    );
    drop(server);
    assert!(open(&legacy, Some(runner)).is_err());
}
