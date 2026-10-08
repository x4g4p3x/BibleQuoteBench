use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

fn isolated_command(program: &str, root: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    command
}

fn git(root: &Path, arguments: &[&str]) {
    let output = isolated_command("git", root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn guard(root: &Path, name: &str) -> Output {
    isolated_command(env!("CARGO_BIN_EXE_biblequotebench"), root)
        .arg(name)
        .output()
        .unwrap()
}

#[test]
fn guards_inspect_index_bytes_even_when_working_files_differ() {
    let temp = TempDir::new().unwrap();
    git(temp.path(), &["init", "--quiet", "--template="]);
    let path = temp.path().join("notes with spaces-é.txt");
    let safe = "OPENAI_API_KEY=YOUR_API_KEY_HERE";
    let token = ["sk-proj-", "abcdefghijklmnopqrstuvwxyz123456"].concat();
    let credential = format!("OPENAI_API_KEY={token}");
    fs::write(&path, safe).unwrap();
    git(temp.path(), &["add", "--force", "--", "."]);
    fs::write(&path, &credential).unwrap();
    for name in ["guard-staged", "guard-tracked"] {
        let output = guard(temp.path(), name);
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("publication guard passed"));
    }
    git(temp.path(), &["add", "--force", "--", "."]);
    fs::write(&path, safe).unwrap();
    for name in ["guard-staged", "guard-tracked"] {
        let output = guard(temp.path(), name);
        assert!(!output.status.success());
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(message.contains("notes with spaces-é.txt"));
        assert!(message.contains("high-confidence credential token"));
        assert!(!message.contains(&token));
        assert!(!String::from_utf8_lossy(&output.stdout).contains(&token));
    }
}

#[test]
fn guards_reject_hidden_paths_and_renamed_hidden_artifacts() {
    let temp = TempDir::new().unwrap();
    git(temp.path(), &["init", "--quiet", "--template="]);
    fs::create_dir_all(temp.path().join("data/hidden")).unwrap();
    fs::write(temp.path().join("data/hidden/cases.jsonl"), "{}").unwrap();
    let identifier = ["BQ-", "HID-", "1234567890ABCDEF"].concat();
    fs::write(
        temp.path().join("public-report.json"),
        serde_json::json!({"case_id": identifier}).to_string(),
    )
    .unwrap();
    git(temp.path(), &["add", "--force", "--", "."]);
    for name in ["guard-staged", "guard-tracked"] {
        let output = guard(temp.path(), name);
        assert!(!output.status.success());
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(message.contains("data/hidden/cases.jsonl"));
        assert!(message.contains("hidden evaluation material"));
        assert!(message.contains("public-report.json"));
        assert!(message.contains("hidden case identifiers"));
        assert!(!message.contains(&identifier));
    }
}

#[test]
fn guards_fail_closed_outside_a_git_repository() {
    let temp = TempDir::new().unwrap();
    for name in ["guard-staged", "guard-tracked"] {
        let output = guard(temp.path(), name);
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Git publication-guard query failed")
        );
    }
}
