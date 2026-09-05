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
    assert!(html.contains("Recall by model configuration"));
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
