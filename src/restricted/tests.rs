use std::thread;

use super::*;
use tempfile::TempDir;

pub(crate) fn fake_program(directory: &Path) -> PathBuf {
    let program = directory.join(if cfg!(windows) {
        "codex-fixture.exe"
    } else {
        "codex-fixture"
    });
    let status = Command::new("rustc")
        .arg("--edition=2024")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/restricted_client.rs"))
        .arg("-o")
        .arg(&program)
        .status()
        .unwrap();
    assert!(status.success());
    program
}

pub(crate) fn fake_runner(
    directory: &Path,
    program: &Path,
    model: &str,
    upstream: &str,
) -> RestrictedRunner {
    let auth_file = directory.join("fixture-auth.json");
    write_json(&auth_file, &json!({"OPENAI_API_KEY":null,"tokens":{"access_token":"fixture","refresh_token":"fixture"}})).unwrap();
    RestrictedRunner {
        program: program.into(),
        auth_file,
        upstream: upstream.into(),
        identity: RunnerIdentity {
            method: "codex_responses_gate_v1".into(),
            model: model.into(),
            cli_version: "codex-cli fixture".into(),
            executable_sha256: hash(&fs::read(program).unwrap()),
            instructions_sha256: hash(INSTRUCTIONS.as_bytes()),
            timeout_seconds: 5,
        },
    }
}

#[allow(clippy::needless_pass_by_value)] // Fixtures accept temporary JSON values.
pub(crate) fn stream(model: &str, parts: Value) -> Vec<u8> {
    let response = json!({"type":"response.completed","response":{"id":"fixture-response","status":"completed","model":model,"output":[{"type":"reasoning","summary":[]},{"type":"message","role":"assistant","content":parts}],"usage":{"input_tokens":15,"output_tokens":27}}});
    format!("event: response.completed\ndata: {response}\n\n").into_bytes()
}

pub(crate) fn upstream(
    model: &str,
    responses: Vec<Vec<u8>>,
) -> (String, thread::JoinHandle<Vec<Value>>) {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}/responses", server.server_addr());
    let model = model.to_owned();
    let worker = thread::spawn(move || {
        let mut seen = Vec::new();
        for response in responses {
            let mut request = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            assert_eq!(request.method(), &Method::Post);
            assert_eq!(request.url(), "/responses");
            let request: Value = {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).unwrap();
                let value: Value = serde_json::from_str(&body).unwrap();
                assert_eq!(value["model"], model);
                assert_eq!(value["tools"], json!([]));
                assert_eq!(value["tool_choice"], "none");
                assert_eq!(value["instructions"], INSTRUCTIONS);
                assert!(!body.contains("CANARY"));
                assert!(value.get("previous_response_id").is_none());
                request
                    .respond(Response::from_data(response).with_header(
                        Header::from_bytes("Content-Type", "text/event-stream").unwrap(),
                    ))
                    .unwrap();
                value
            };
            seen.push(request);
        }
        seen
    });
    (url, worker)
}

#[test]
fn canonical_request_has_no_client_context_tools_or_history() {
    let request = canonical_request("Quote this passage", "fixture");
    assert_eq!(
        request["input"],
        json!([{"role":"user","content":[{"type":"input_text","text":"Quote this passage"}]}])
    );
    assert_eq!(request["tools"], json!([]));
    assert_eq!(request["tool_choice"], "none");
    assert_eq!(request["parallel_tool_calls"], false);
    assert_eq!(request["store"], false);
    assert_eq!(request["reasoning"]["effort"], "low");
    assert!(request.get("previous_response_id").is_none());
}

#[test]
fn text_refusal_empty_and_unicode_boundaries_are_preserved() {
    for parts in [
        json!([]),
        json!([{"type":"output_text","text":"  A\nB  "}]),
        json!([{"type":"refusal","refusal":""}]),
        json!([{"type":"output_text","text":"part"},{"type":"refusal","refusal":"refusal"}]),
    ] {
        let result = validate_response(&stream("fixture", parts.clone()), "fixture").unwrap();
        assert_eq!(result.execution.input_tokens, Some(15));
        assert_eq!(result.execution.output_tokens, Some(27));
        assert_eq!(
            result.execution.refusal,
            parts
                .as_array()
                .unwrap()
                .iter()
                .any(|part| part["type"] == "refusal")
        );
        assert_eq!(
            result.text,
            parts
                .as_array()
                .unwrap()
                .iter()
                .map(|part| part["text"]
                    .as_str()
                    .or_else(|| part["refusal"].as_str())
                    .unwrap())
                .collect::<String>()
        );
    }
    let exact = "𝄞".repeat(16_384);
    assert_eq!(
        validate_response(
            &stream("fixture", json!([{"type":"output_text","text":exact}])),
            "fixture"
        )
        .unwrap()
        .text,
        exact
    );
    assert!(
        validate_response(
            &stream("fixture", json!([{"type":"output_text","text":exact+"x"}])),
            "fixture"
        )
        .is_err()
    );
}

#[test]
fn every_tool_family_and_unknown_stream_type_is_rejected_before_delivery() {
    for kind in [
        "function_call",
        "custom_tool_call",
        "web_search_call",
        "computer_call",
        "code_interpreter_call",
        "mcp_call",
        "file_search_call",
        "image_generation_call",
        "future_tool",
    ] {
        let added = format!(
            "data: {}\n\n",
            json!({"type":"response.output_item.added","item":{"type":kind}})
        );
        let mut bytes = added.into_bytes();
        bytes.extend(stream("fixture", json!([])));
        assert!(
            validate_response(&bytes, "fixture")
                .unwrap_err()
                .to_string()
                .contains("tool use")
        );
    }
    for kind in [
        "response.function_call_arguments.delta",
        "response.failed",
        "response.incomplete",
        "error",
        "future.event",
    ] {
        let bytes = format!("data: {}\n\n", json!({"type":kind}));
        assert!(validate_response(bytes.as_bytes(), "fixture").is_err());
    }
    let mut bytes = format!(
        "data: {}\n\n",
        json!({"type":"response.content_part.added","part":{"type":"computer_call"}})
    )
    .into_bytes();
    bytes.extend(stream("fixture", json!([])));
    assert!(validate_response(&bytes, "fixture").is_err());
}

#[test]
fn malformed_incomplete_wrong_model_and_duplicate_completions_are_rejected() {
    for bytes in [
        vec![255],
        b"data: invalid\n\n".to_vec(),
        b"data: {}\n\n".to_vec(),
        b"data: [DONE]\n\n".to_vec(),
    ] {
        assert!(validate_response(&bytes, "fixture").is_err());
    }
    let base = stream("fixture", json!([{"type":"output_text","text":"text"}]));
    assert!(validate_response(&base, "another-model").is_err());
    let mut duplicated = base.clone();
    duplicated.extend(&base);
    assert!(validate_response(&duplicated, "fixture").is_err());
    let event: Value = serde_json::from_str(
        std::str::from_utf8(&base)
            .unwrap()
            .lines()
            .nth(1)
            .unwrap()
            .strip_prefix("data: ")
            .unwrap(),
    )
    .unwrap();
    for (pointer, value) in [
        ("/response/status", json!("incomplete")),
        ("/response/usage/input_tokens", json!(-1)),
        ("/response/usage/output_tokens", Value::Null),
        ("/response/output/1/role", json!("user")),
        ("/response/output/1/content/0/type", json!("image")),
        ("/response/output/1/content/0/text", Value::Null),
        ("/response/output", Value::Null),
    ] {
        let mut changed = event.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert!(validate_response(format!("data: {changed}\n\n").as_bytes(), "fixture").is_err());
    }
    let crlf = String::from_utf8(base).unwrap().replace('\n', "\r\n");
    assert_eq!(
        validate_response(crlf.as_bytes(), "fixture").unwrap().text,
        "text"
    );
}

#[test]
fn only_subscription_credentials_are_copied_and_files_are_not_overwritten() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("auth.json");
    assert!(subscription_auth(&path).is_err());
    fs::write(&path, b"malformed").unwrap();
    assert!(subscription_auth(&path).is_err());
    for value in [
        json!({"auth_mode":"apikey","tokens":{"access_token":"fixture","refresh_token":"fixture"}}),
        json!({"OPENAI_API_KEY":"fixture"}),
        json!({"tokens":{}}),
    ] {
        write_json(&path, &value).unwrap();
        assert!(subscription_auth(&path).is_err());
    }
    for mode in [false, true] {
        let mut value = json!({"tokens":{"access_token":"fixture","refresh_token":"fixture"},"OPENAI_API_KEY":null});
        if mode {
            value["auth_mode"] = json!("chatgpt");
        }
        write_json(&path, &value).unwrap();
        assert_eq!(subscription_auth(&path).unwrap(), value);
    }
    let copy = temp.path().join("copy.json");
    private_auth(&copy, &json!({"fixture":true})).unwrap();
    assert!(private_auth(&copy, &json!({"fixture":false})).is_err());
    assert_eq!(fs::read(&copy).unwrap(), br#"{"fixture":true}"#);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(copy).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn refreshed_credentials_are_retained_without_overwriting_account_or_source_changes() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source.json");
    let isolated = temp.path().join("isolated.json");
    let original = json!({"OPENAI_API_KEY":null,"tokens":{"access_token":"fixture","refresh_token":"original","account_id":"account"}});
    write_json(&source, &original).unwrap();
    write_json(&isolated, &original).unwrap();
    persist_auth_refresh(&source, &isolated, &original).unwrap();
    let mut refreshed = original.clone();
    refreshed["tokens"]["refresh_token"] = json!("refreshed");
    write_json(&isolated, &refreshed).unwrap();
    persist_auth_refresh(&source, &isolated, &original).unwrap();
    assert_eq!(subscription_auth(&source).unwrap(), refreshed);
    assert!(persist_auth_refresh(&source, &isolated, &original).is_err());
    assert_eq!(subscription_auth(&source).unwrap(), refreshed);
    write_json(&source, &original).unwrap();
    refreshed["tokens"]["account_id"] = json!("other-account");
    write_json(&isolated, &refreshed).unwrap();
    assert!(persist_auth_refresh(&source, &isolated, &original).is_err());
    assert_eq!(subscription_auth(&source).unwrap(), original);
    fs::write(&isolated, "invalid").unwrap();
    assert!(persist_auth_refresh(&source, &isolated, &original).is_err());
}

fn deliver_request(
    method: Method,
    path: &str,
    body: Vec<u8>,
    headers: Vec<(&'static str, &'static str)>,
    upstream_url: &str,
) -> String {
    let temp = TempDir::new().unwrap();
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}{path}", server.server_addr());
    let sender = thread::spawn(move || {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let mut request = client
            .request(method.as_str().parse().unwrap(), url)
            .body(body);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request.send().unwrap();
        assert_eq!(response.status().as_u16(), 502);
        assert_eq!(
            response.text().unwrap(),
            "Restricted recall gate rejected the request or response"
        );
    });
    let request = server
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let error = forward(
        request,
        &client,
        "/responses",
        upstream_url,
        "canonical",
        "fixture",
        temp.path(),
    )
    .unwrap_err()
    .to_string();
    assert!(!error.contains("CANARY"));
    sender.join().unwrap();
    error
}

#[test]
fn malformed_http_requests_are_rejected_without_upstream_calls_or_sensitive_errors() {
    let valid = serde_json::to_vec(&json!({"model":"fixture","stream":true})).unwrap();
    for (method, path, body, headers) in [
        (Method::Get, "/responses", valid.clone(), vec![]),
        (Method::Post, "/wrong", valid.clone(), vec![]),
        (Method::Post, "/responses", b"invalid".to_vec(), vec![]),
        (
            Method::Post,
            "/responses",
            valid.clone(),
            vec![("Content-Encoding", "gzip")],
        ),
        (
            Method::Post,
            "/responses",
            serde_json::to_vec(&json!({"model":"wrong","stream":true})).unwrap(),
            vec![],
        ),
        (
            Method::Post,
            "/responses",
            serde_json::to_vec(&json!({"model":"fixture","stream":false})).unwrap(),
            vec![],
        ),
        (
            Method::Post,
            "/responses",
            vec![b' '; usize::try_from(MAX_REQUEST + 1).unwrap()],
            vec![],
        ),
        (
            Method::Post,
            "/responses",
            valid,
            vec![("X-Untrusted", "CANARY")],
        ),
    ] {
        deliver_request(method, path, body, headers, "http://127.0.0.1:9");
    }
}

#[test]
fn upstream_errors_redirects_size_limits_and_header_filtering_fail_closed() {
    for (status, body) in [
        (500, b"CANARY private error".to_vec()),
        (302, b"redirect".to_vec()),
        (200, vec![b' '; usize::try_from(MAX_RESPONSE + 1).unwrap()]),
    ] {
        let upstream_server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/responses", upstream_server.server_addr());
        let worker = thread::spawn(move || {
            let request = upstream_server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            assert!(
                request
                    .headers()
                    .iter()
                    .any(|h| h.field.equiv("Authorization") && h.value == "Bearer fixture")
            );
            assert!(
                !request
                    .headers()
                    .iter()
                    .any(|h| h.field.equiv("X-Untrusted"))
            );
            let _ = request.respond(
                Response::from_data(body)
                    .with_status_code(StatusCode(status))
                    .with_header(Header::from_bytes("Location", "http://127.0.0.1:9").unwrap()),
            );
        });
        let error = deliver_request(
            Method::Post,
            "/responses",
            serde_json::to_vec(&json!({"model":"fixture","stream":true})).unwrap(),
            vec![
                ("Authorization", "Bearer fixture"),
                ("X-Untrusted", "CANARY"),
            ],
            &url,
        );
        assert!(error.contains(if status == 200 { "size limit" } else { "HTTP" }));
        worker.join().unwrap();
    }
}

#[test]
fn runner_configuration_and_executable_changes_fail_before_model_requests() {
    for model in ["", "model with spaces", "model;command"] {
        assert!(
            RestrictedRunner::prepare(&RunnerConfig {
                program: PathBuf::from("missing"),
                model: model.into(),
                timeout_seconds: 5
            })
            .is_err()
        );
    }
    assert!(
        RestrictedRunner::prepare(&RunnerConfig {
            program: PathBuf::from("missing"),
            model: "fixture".into(),
            timeout_seconds: 0
        })
        .is_err()
    );
    assert!(resolve_program(Path::new("missing-bqb-program")).is_err());
    let temp = TempDir::new().unwrap();
    let program = temp.path().join("program");
    fs::write(&program, b"original").unwrap();
    let runner = fake_runner(temp.path(), &program, "fixture", "http://127.0.0.1:9");
    fs::write(&program, b"changed").unwrap();
    assert!(
        runner
            .recall("prompt", &temp.path().join("audit"))
            .unwrap_err()
            .to_string()
            .contains("executable changed")
    );
}

#[test]
fn real_local_process_uses_gate_rewrites_context_and_validates_audit() {
    let temp = TempDir::new().unwrap();
    let program = fake_program(temp.path());
    for model in ["fixture", "catalogue"] {
        let (url, worker) = upstream(
            model,
            vec![stream(
                model,
                json!([{"type":"output_text","text":"  kept\nexactly  "}]),
            )],
        );
        let runner = fake_runner(temp.path(), &program, model, &url);
        let audit = temp.path().join(model);
        let result = runner.recall("canonical prompt", &audit).unwrap();
        assert_eq!(result.text, "  kept\nexactly  ");
        assert_eq!(
            worker.join().unwrap()[0],
            canonical_request("canonical prompt", model)
        );
        assert_eq!(
            read_audit(&audit, "canonical prompt", model)
                .unwrap()
                .response_sha256,
            result.response_sha256
        );
        let original = fs::read(audit.join("request.json")).unwrap();
        let mut changed: Value = serde_json::from_slice(&original).unwrap();
        changed["tools"] = json!([{"type":"web_search"}]);
        write_json(&audit.join("request.json"), &changed).unwrap();
        assert!(read_audit(&audit, "canonical prompt", model).is_err());
        fs::write(audit.join("request.json"), original).unwrap();
        fs::write(audit.join("response.sse"), b"data: broken\n\n").unwrap();
        assert!(read_audit(&audit, "canonical prompt", model).is_err());
    }
}

#[test]
fn process_failures_forbidden_output_extra_requests_and_timeouts_stop_without_retry() {
    let temp = TempDir::new().unwrap();
    let program = fake_program(temp.path());
    for model in ["fail", "route", "hang"] {
        let mut runner = fake_runner(temp.path(), &program, model, "http://127.0.0.1:9");
        runner.identity.timeout_seconds = 1;
        assert!(runner.recall("prompt", &temp.path().join(model)).is_err());
    }
    let (url, worker) = upstream("repeat", vec![stream("repeat", json!([]))]);
    assert!(
        fake_runner(temp.path(), &program, "repeat", &url)
            .recall("prompt", &temp.path().join("repeat"))
            .unwrap_err()
            .to_string()
            .contains("more than one")
    );
    assert_eq!(worker.join().unwrap().len(), 1);
    let forbidden = format!(
        "data: {}\n\n",
        json!({"type":"response.output_item.added","item":{"type":"web_search_call"}})
    )
    .into_bytes();
    let (url, worker) = upstream("fixture", vec![forbidden]);
    assert!(
        fake_runner(temp.path(), &program, "fixture", &url)
            .recall("prompt", &temp.path().join("forbidden"))
            .unwrap_err()
            .to_string()
            .contains("tool use")
    );
    assert_eq!(worker.join().unwrap().len(), 1);
}
