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
