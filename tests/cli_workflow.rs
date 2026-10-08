use std::{fs, path::PathBuf, process::Command};

use serde_json::Value;
use tempfile::TempDir;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(arguments: &[&str]) -> std::process::Output {
    let output = Command::new(env!("CARGO_BIN_EXE_biblequotebench"))
        .current_dir(project_root())
        .args(arguments)
        .output()
        .expect("benchmark command should start");
    assert!(
        output.status.success(),
        "command failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn public_dataset_validates_and_prompts() {
    let validation = run(&["validate"]);
    assert!(String::from_utf8_lossy(&validation.stdout).contains("300 cases"));

    let first_case: Value = serde_json::from_str(
        fs::read_to_string(project_root().join("data/dev/cases.jsonl"))
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    let case_id = first_case["case_id"].as_str().unwrap();
    let prompt = run(&["prompt", "--case-id", case_id]);
    let prompt = String::from_utf8(prompt.stdout).unwrap();
    assert!(prompt.contains("Output only the passage text."));
    assert!(prompt.contains("2025 third printing"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn restricted_cli_and_stdio_block_failed_attempts_without_network_or_api_keys() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::Stdio;
    let temp = TempDir::new().unwrap();
    let program = temp.path().join(if cfg!(windows) {
        "fixture.exe"
    } else {
        "fixture"
    });
    assert!(
        Command::new("rustc")
            .arg("--edition=2024")
            .arg(project_root().join("tests/support/restricted_client.rs"))
            .arg("-o")
            .arg(&program)
            .status()
            .unwrap()
            .success()
    );
    let auth_path = temp.path().join("auth.json");
    fs::write(
        &auth_path,
        r#"{"OPENAI_API_KEY":null,"tokens":{"access_token":"fixture","refresh_token":"fixture"}}"#,
    )
    .unwrap();
    let mut arguments = vec![
        "restricted",
        "--model",
        "fail",
        "--run-id",
        "blocked",
        "--case-limit",
        "1",
        "--codex-bin",
        program.to_str().unwrap(),
        "--output-dir",
        temp.path().to_str().unwrap(),
    ];
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_biblequotebench"))
            .current_dir(project_root())
            .env("CODEX_HOME", temp.path())
            .args(args)
            .output()
            .unwrap()
    };
    let failed = invoke(&arguments);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("runner failed"));
    let checkpoint = temp.path().join("run-blocked/session.json");
    let saved = fs::read(&checkpoint).unwrap();
    let session: Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(session["restricted_attempts"][0]["state"], "blocked");
    assert_eq!(session["responses"], serde_json::json!([]));
    assert_eq!(
        session["identity"]["restriction"]["reasoning_effort"],
        "low"
    );
    assert!(!invoke(&arguments).status.success()); // Explicit resume required.
    arguments.push("--resume");
    let mut changed_effort = arguments.clone();
    changed_effort.extend(["--reasoning-effort", "xhigh"]);
    let rejected = invoke(&changed_effort);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("resume settings or dataset changed")
    );
    assert!(!invoke(&arguments).status.success()); // No reexecution of an uncertain case.
    assert_eq!(fs::read(&checkpoint).unwrap(), saved);
    arguments[4] = "missing";
    assert!(!invoke(&arguments).status.success());
    assert!(!temp.path().join("run-missing").exists());

    let mut child = Command::new(env!("CARGO_BIN_EXE_biblequotebench"))
        .current_dir(project_root())
        .env("CODEX_HOME", temp.path())
        .args([
            "mcp",
            "--restricted-model",
            "fail",
            "--restricted-reasoning-effort",
            "xhigh",
            "--run-id",
            "stdio",
            "--model",
            "fail",
            "--case-limit",
            "1",
            "--translation",
            "bsb-2025-third-printing",
            "--codex-bin",
            program.to_str().unwrap(),
            "--output-dir",
            temp.path().to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    writeln!(input, "{}", serde_json::json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}})).unwrap();
    input.flush().unwrap();
    let mut initialized = String::new();
    output.read_line(&mut initialized).unwrap();
    assert!(
        serde_json::from_str::<Value>(&initialized)
            .unwrap()
            .get("error")
            .is_none()
    );
    writeln!(
        input,
        "{}",
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    let mut rpc = |id: u32, name: &str, args: Value| {
        writeln!(input, "{}", serde_json::json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}})).unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let reply: Value = serde_json::from_str(&line).unwrap();
        reply["result"].clone()
    };
    let next = rpc(1, "next_case", serde_json::json!({}));
    let issued: Value = serde_json::from_str(next["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(issued.get("prompt").is_none());
    let failed = rpc(
        2,
        "answer_case",
        serde_json::json!({"case_id":issued["case_id"]}),
    );
    assert_eq!(failed["isError"], true);
    assert_eq!(
        rpc(
            3,
            "submit_answer",
            serde_json::json!({"case_id":issued["case_id"],"output":"copied answer"})
        )["isError"],
        true
    );
    drop(rpc);
    drop(input);
    assert!(child.wait().unwrap().success());
    let saved: Value =
        serde_json::from_slice(&fs::read(temp.path().join("run-stdio/session.json")).unwrap())
            .unwrap();
    assert_eq!(saved["restricted_attempts"][0]["state"], "blocked");
    assert_eq!(saved["identity"]["evidence"], "restricted_codex");
    assert_eq!(
        saved["identity"]["restriction"]["reasoning_effort"],
        "xhigh"
    );

    fs::write(auth_path, r#"{"OPENAI_API_KEY":"fixture"}"#).unwrap();
    arguments.truncate(arguments.len() - 1);
    let rejected = invoke(&arguments);
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("API-key authentication is rejected")
    );
    assert!(!temp.path().join("run-missing").exists());
}

#[test]
fn scoring_summary_and_report_work_end_to_end() {
    let output = TempDir::new().unwrap();
    let scores = output.path().join("scores.jsonl");
    let report_markdown = output.path().join("report.md");
    let report_json = output.path().join("report.json");

    run(&[
        "score",
        "--responses",
        "data/dev/responses.example.jsonl",
        "--output",
        scores.to_str().unwrap(),
    ]);
    let summary = run(&["summarize", "--scores", scores.to_str().unwrap()]);
    let summary: Value = serde_json::from_slice(&summary.stdout).unwrap();
    assert_eq!(summary["responses"], 3);
    assert_eq!(summary["classifications"]["exact_requested"], 1);
    assert_eq!(summary["classifications"]["translation_confusion"], 1);

    run(&[
        "report",
        "--scores",
        scores.to_str().unwrap(),
        "--markdown",
        report_markdown.to_str().unwrap(),
        "--json",
        report_json.to_str().unwrap(),
    ]);
    assert!(
        fs::read_to_string(report_markdown)
            .unwrap()
            .contains("## Requested → resembles")
    );
    let report: Value = serde_json::from_slice(&fs::read(report_json).unwrap()).unwrap();
    assert_eq!(report["overall"]["responses"], 3);
}

#[test]
fn usfm_import_command_emits_corpus_and_provenance_lock() {
    let output = TempDir::new().unwrap();
    let source = output.path().join("usfm");
    fs::create_dir(&source).unwrap();
    fs::write(
        source.join("01GEN.usfm"),
        "\\id GEN\n\\c 1\n\\v 1 In the beginning.\n\\v 2 The second verse.\n",
    )
    .unwrap();
    let catalog = output.path().join("translations.json");
    fs::write(
        &catalog,
        r#"{"schema_version":1,"translations":[{"id":"fixture","name":"Fixture","abbreviation":"FIX","edition":"1","license_kind":"public_domain","license_url":"https://example.test/license","source_url":"https://example.test/source","redistribute_reference_text":true}]}"#,
    )
    .unwrap();
    let corpus = output.path().join("corpus.jsonl");
    let lock = output.path().join("lock.json");
    run(&[
        "import-usfm",
        "--translations",
        catalog.to_str().unwrap(),
        "--translation",
        "fixture",
        "--source",
        source.to_str().unwrap(),
        "--output",
        corpus.to_str().unwrap(),
        "--lock-output",
        lock.to_str().unwrap(),
    ]);
    assert_eq!(fs::read_to_string(corpus).unwrap().lines().count(), 2);
    let lock: Value = serde_json::from_slice(&fs::read(lock).unwrap()).unwrap();
    assert_eq!(lock["reference_count"], 2);
    assert_eq!(lock["translation"], "fixture");
}

fn diagnostic_fixture(root: &std::path::Path) -> (PathBuf, PathBuf, PathBuf) {
    let translations = root.join("translations.json");
    let cases_path = root.join("cases.jsonl");
    let references_path = root.join("references.jsonl");
    let catalog = serde_json::json!({"schema_version":1,"translations": (["a", "b"].map(|id| serde_json::json!({"id":id,"name":id,"abbreviation":id,"edition":"1","license_kind":"public_domain","license_url":"https://example.test/license","source_url":"https://example.test/source","redistribute_reference_text":true})))});
    fs::write(&translations, catalog.to_string()).unwrap();
    let mut cases = String::new();
    let mut references = String::new();
    for verse in 1..=12 {
        for translation in ["a", "b"] {
            let reference = serde_json::json!({"book":"John","chapter":1,"verse_start":verse});
            references.push_str(&serde_json::json!({"translation":translation,"reference":reference,"text":format!("The {translation} verse {verse}.")}).to_string());
            references.push('\n');
            if [1, 4].contains(&verse) {
                cases.push_str(&serde_json::json!({"case_id":format!("BQ-DEV-{verse}-{translation}"),"translation":translation,"reference":reference,"stratum":"random","prompt_variant":"canonical"}).to_string());
                cases.push('\n');
            }
        }
    }
    fs::write(&cases_path, cases).unwrap();
    fs::write(&references_path, references).unwrap();
    (translations, cases_path, references_path)
}

fn sampling_arguments(root: &std::path::Path) -> Vec<String> {
    let (catalog, _, references) = diagnostic_fixture(root);
    let records: Vec<biblequotebench::ReferenceRecord> =
        biblequotebench::io::read_jsonl(&references).unwrap();
    let config = root.join("sampling.json");
    fs::write(&config, serde_json::json!({"schema_version":2,"release_id":"fixture","seed":"public-seed","total_references":6,"dev_references":2,"famous_references":0,"translation_sensitive_references":0,"short_references":0,"long_references":0}).to_string()).unwrap();
    let curated = root.join("curated.jsonl");
    fs::write(&curated, "\n").unwrap();
    let mut arguments = vec!["sample".into()];
    for (flag, path) in [
        ("--config", config),
        ("--translations", catalog),
        ("--curated", curated),
        ("--hidden-seed-file", root.join("private-seed.txt")),
        ("--dev-cases", root.join("dev/cases.jsonl")),
        ("--dev-references", root.join("dev/references.jsonl")),
        ("--hidden-cases", root.join("hidden/cases.jsonl")),
        ("--hidden-references", root.join("hidden/references.jsonl")),
        ("--manifest", root.join("release/manifest.json")),
    ] {
        arguments.extend([flag.into(), path.to_str().unwrap().into()]);
    }
    for translation in ["a", "b"] {
        let corpus = root.join(format!("corpus-{translation}.jsonl"));
        let selected: Vec<_> = records
            .iter()
            .filter(|record| record.translation == translation)
            .collect();
        biblequotebench::io::write_jsonl(Some(&corpus), &selected).unwrap();
        let lock = root.join(format!("lock-{translation}.json"));
        fs::write(&lock, serde_json::json!({"schema_version":1,"translation":translation,"edition":"1","source_url":"https://example.test/fixture","source_sha256":"fixture","importer_version":"fixture","artifacts":[],"reference_count":12,"corpus_sha256":biblequotebench::importer::sha256_hex(&fs::read(&corpus).unwrap())}).to_string()).unwrap();
        arguments.extend([
            "--corpus".into(),
            corpus.to_str().unwrap().into(),
            "--lock".into(),
            lock.to_str().unwrap().into(),
        ]);
    }
    arguments
}

#[test]
fn sampling_command_preserves_public_split_and_private_seed_boundaries() {
    let temp = TempDir::new().unwrap();
    let arguments = sampling_arguments(temp.path());
    let args: Vec<_> = arguments.iter().map(String::as_str).collect();
    let missing = Command::new(env!("CARGO_BIN_EXE_biblequotebench"))
        .args(&arguments)
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("reading private hidden seed"));
    assert!(!temp.path().join("dev").exists());
    let seed = temp.path().join("private-seed.txt");
    fs::write(&seed, "private-first\n").unwrap();
    run(&args);
    let manifest_path = temp.path().join("release/manifest.json");
    let first: Value = biblequotebench::io::read_json(&manifest_path).unwrap();
    assert_eq!(first["dev_case_count"], 4);
    assert_eq!(first["hidden_case_count"], 8);
    let public: Vec<biblequotebench::BenchmarkCase> =
        biblequotebench::io::read_jsonl(&temp.path().join("dev/cases.jsonl")).unwrap();
    let hidden: Vec<biblequotebench::BenchmarkCase> =
        biblequotebench::io::read_jsonl(&temp.path().join("hidden/cases.jsonl")).unwrap();
    assert!(
        public
            .iter()
            .all(|case| hidden.iter().all(|other| case.reference != other.reference))
    );
    assert!(!first.to_string().contains("private-first"));
    run(&args);
    assert_eq!(
        first,
        biblequotebench::io::read_json::<Value>(&manifest_path).unwrap()
    );
    fs::write(&seed, "private-second\n").unwrap();
    run(&args);
    let second: Value = biblequotebench::io::read_json(&manifest_path).unwrap();
    assert_eq!(first["dev_cases_sha256"], second["dev_cases_sha256"]);
    assert_ne!(first["hidden_cases_sha256"], second["hidden_cases_sha256"]);
}

#[test]
fn scoring_to_stdout_and_invalid_inputs_have_useful_diagnostics() {
    let scored = run(&["score", "--responses", "data/dev/responses.example.jsonl"]);
    let lines: Vec<Value> = String::from_utf8(scored.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["classification"], "exact_requested");
    let temp = TempDir::new().unwrap();
    let empty = temp.path().join("empty.jsonl");
    fs::write(&empty, "\n").unwrap();
    for args in [
        vec!["score", "--responses", empty.to_str().unwrap()],
        vec!["summarize", "--scores", empty.to_str().unwrap()],
        vec!["report", "--scores", empty.to_str().unwrap()],
        vec!["prompt", "--case-id", "unknown-case"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_biblequotebench"))
            .current_dir(project_root())
            .args(&args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let message = String::from_utf8_lossy(&output.stderr);
        if args[0] == "prompt" {
            assert!(message.contains("unknown case_id"));
        } else {
            assert!(message.contains("contains no"));
        }
    }
    let unknown = temp.path().join("unknown.jsonl");
    let mut response: Value = serde_json::from_str(
        fs::read_to_string(project_root().join("data/dev/responses.example.jsonl"))
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    response["case_id"] = serde_json::json!("unknown-case");
    fs::write(&unknown, response.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_biblequotebench"))
        .current_dir(project_root())
        .args(["score", "--responses", unknown.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown case_id"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn diagnostic_tracks_synthetic_pilot_and_validated_analysis_work_end_to_end() {
    let temp = TempDir::new().unwrap();
    let (catalog, cases, references) = diagnostic_fixture(temp.path());
    let datasets = temp.path().join("datasets");
    let output = temp.path().join("pilot");
    run(&[
        "prepare-pilot",
        "--translations",
        catalog.to_str().unwrap(),
        "--cases",
        cases.to_str().unwrap(),
        "--references",
        references.to_str().unwrap(),
        "--corpus",
        references.to_str().unwrap(),
        "--reference-count",
        "2",
        "--output-dir",
        datasets.to_str().unwrap(),
    ]);
    let plan: Value =
        serde_json::from_slice(&fs::read(datasets.join("live-plan.json")).unwrap()).unwrap();
    assert_eq!(plan["execution_enabled"], false);
    assert_eq!(plan["budget_eur"], 20);
    assert_eq!(plan["spent_eur"], 0);
    assert_eq!(plan["requested_reasoning_effort"], "max");
    assert_eq!(plan["requested_reasoning_verified"], true);
    run(&[
        "synthetic-pilot",
        "--dataset-dir",
        datasets.to_str().unwrap(),
        "--output-dir",
        output.to_str().unwrap(),
    ]);
    for track in biblequotebench::pilot::TRACKS {
        let report: Value =
            serde_json::from_slice(&fs::read(output.join(track).join("analysis.json")).unwrap())
                .unwrap();
        assert_eq!(report["evidence"], "synthetic_fixture");
        assert_eq!(report["track"], track);
        assert_eq!(report["models"].as_object().unwrap().len(), 2);
        for model in report["models"].as_object().unwrap().values() {
            assert_eq!(model["repetitions"], 3);
            assert_eq!(model["exact_text"]["clusters"], 2);
        }
    }
    let track = datasets.join("canonical");
    let responses = output.join("canonical/synthetic-a-0.jsonl");
    let analyzed = temp.path().join("analysis");
    run(&[
        "analyze",
        "--translations",
        track.join("translations.json").to_str().unwrap(),
        "--cases",
        track.join("cases.jsonl").to_str().unwrap(),
        "--references",
        track.join("references.jsonl").to_str().unwrap(),
        "--responses",
        responses.to_str().unwrap(),
        "--output-dir",
        analyzed.to_str().unwrap(),
        "--resamples",
        "100",
    ]);
    assert!(analyzed.join("analysis.md").exists());
    assert!(analyzed.join("analysis.html").exists());
    let interactive = temp.path().join("standalone/report.html");
    run(&[
        "visualize",
        "--analysis",
        analyzed.join("analysis.json").to_str().unwrap(),
        "--output",
        interactive.to_str().unwrap(),
    ]);
    let html = fs::read_to_string(&interactive).unwrap();
    assert!(html.contains("Quotation accuracy by model"));
    assert!(html.contains("connect-src 'none'"));
    assert!(output.join("index.html").exists());
    let copy = datasets.join("copy_control");
    let first: Value = serde_json::from_str(
        fs::read_to_string(copy.join("cases.jsonl"))
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    let prompt = run(&[
        "prompt",
        "--translations",
        copy.join("translations.json").to_str().unwrap(),
        "--cases",
        copy.join("cases.jsonl").to_str().unwrap(),
        "--references",
        copy.join("references.jsonl").to_str().unwrap(),
        "--case-id",
        first["case_id"].as_str().unwrap(),
    ]);
    assert!(
        String::from_utf8(prompt.stdout)
            .unwrap()
            .contains("<supplied_text>")
    );
    assert!(
        fs::read_to_string(output.join("README.md"))
            .unwrap()
            .contains("Synthetic validation only")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn live_runner_manifest_and_copy_boundary_are_checked_over_loopback_only() {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    let temp = TempDir::new().unwrap();
    let (catalog, cases, references) = diagnostic_fixture(temp.path());
    for copy in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse().unwrap())
                        })
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let body = r#"{"id":"test-request","model":"fixture-version","output":[{"content":[{"type":"output_text","text":"The a verse 1."}]}]}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            String::from_utf8(request).unwrap()
        });
        if copy {
            let text = fs::read_to_string(&cases)
                .unwrap()
                .replace("canonical", "copy_control");
            fs::write(&cases, text).unwrap();
        }
        let output = temp.path().join(format!("response-{copy}.jsonl"));
        let result = Command::new(env!("CARGO_BIN_EXE_biblequotebench"))
            .current_dir(project_root())
            .env("BQB_LOOPBACK_KEY", "loopback-fixture")
            .args([
                "run",
                "--provider",
                "openai",
                "--api-key-env",
                "BQB_LOOPBACK_KEY",
                "--base-url",
                &url,
                "--model",
                "fixture",
                "--run-id",
                "run-1",
                "--temperature",
                "0",
                "--reasoning-effort",
                "high",
                "--case-limit",
                "1",
                "--translations",
                catalog.to_str().unwrap(),
                "--cases",
                cases.to_str().unwrap(),
                "--references",
                references.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let request = server.join().unwrap();
        assert_eq!(request.contains("<supplied_text>"), copy);
        assert_eq!(request.contains("The a verse 1."), copy);
        assert!(request.contains(r#""reasoning":{"effort":"high"}"#));
        let manifest: Value = serde_json::from_slice(
            &fs::read(biblequotebench::study::manifest_path(&output)).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["expected_case_ids"].as_array().unwrap().len(), 4);
        assert_eq!(manifest["reasoning_effort"], "high");
        let analysis = Command::new(env!("CARGO_BIN_EXE_biblequotebench"))
            .current_dir(project_root())
            .args([
                "analyze",
                "--translations",
                catalog.to_str().unwrap(),
                "--cases",
                cases.to_str().unwrap(),
                "--references",
                references.to_str().unwrap(),
                "--responses",
                output.to_str().unwrap(),
                "--output-dir",
                temp.path().join("analysis").to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(!analysis.status.success());
        assert!(String::from_utf8_lossy(&analysis.stderr).contains("incomplete"));
    }
}

#[test]
fn killed_runner_resumes_without_replaying_an_uncertain_paid_request() {
    use std::{
        io::Read as _,
        net::TcpListener,
        process::Stdio,
        time::{Duration, Instant},
    };
    let temp = TempDir::new().unwrap();
    let (catalog, cases, references) = diagnostic_fixture(temp.path());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let output = temp.path().join("interrupted.jsonl");
    let command = || {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_biblequotebench"));
        cmd.current_dir(project_root()).args([
            "run",
            "--provider",
            "openai-compatible",
            "--model",
            "fixture",
            "--base-url",
            &url,
            "--run-id",
            "interruption-test",
            "--case-limit",
            "1",
            "--translations",
            catalog.to_str().unwrap(),
            "--cases",
            cases.to_str().unwrap(),
            "--references",
            references.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ]);
        cmd
    };
    let mut child = command()
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut stream = loop {
        if let Ok((stream, _)) = listener.accept() {
            break stream;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("runner did not issue the expected loopback request");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = [0; 4096];
    assert!(stream.read(&mut bytes).unwrap() > 0);
    child.kill().unwrap();
    child.wait().unwrap();
    drop(stream);
    drop(listener);
    let resumed = command().arg("--resume").output().unwrap();
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let records: Vec<biblequotebench::ResponseRecord> =
        biblequotebench::io::read_jsonl(&output).unwrap();
    assert_eq!(records.len(), 1);
    assert!(
        records[0]
            .error
            .as_deref()
            .unwrap()
            .contains("not replayed")
    );
    assert!(records[0].execution.as_ref().unwrap().reservation_retained);
    assert!(
        records[0]
            .execution
            .as_ref()
            .unwrap()
            .accounted_nanoeur
            .unwrap()
            > 0
    );
    assert!(biblequotebench::study::manifest_path(&output).exists());
}
