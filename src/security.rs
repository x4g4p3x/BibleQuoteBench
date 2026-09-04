//! Publication guards for Git-indexed hidden data and credential material.

use std::{path::Path, process::Command, sync::OnceLock};

use anyhow::{Context, Result, bail};
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardViolation {
    pub path: String,
    pub reason: &'static str,
}

/// Checks files changed in the Git index for private benchmark material and credentials.
///
/// # Errors
///
/// Returns an error when Git cannot be queried or a staged entry cannot be read.
pub fn guard_staged() -> Result<Vec<GuardViolation>> {
    let paths = git_paths(&[
        "diff",
        "--cached",
        "--name-only",
        "-z",
        "--diff-filter=ACMR",
    ])?;
    inspect_index_paths(&paths)
}

/// Checks every file in the Git index for private benchmark material and credentials.
///
/// This is intended for CI, where the checked-out index represents the repository.
///
/// # Errors
///
/// Returns an error when Git cannot be queried or an indexed entry cannot be read.
pub fn guard_tracked() -> Result<Vec<GuardViolation>> {
    let paths = git_paths(&["ls-files", "--cached", "-z"])?;
    inspect_index_paths(&paths)
}

fn git_paths(arguments: &[&str]) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(arguments)
        .output()
        .context("running Git for the publication guard")?;
    if !output.status.success() {
        bail!(
            "Git publication-guard query failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            String::from_utf8(path.to_vec()).context("Git returned a non-UTF-8 repository path")
        })
        .collect()
}

fn inspect_index_paths(paths: &[String]) -> Result<Vec<GuardViolation>> {
    let mut violations = Vec::new();
    for path in paths {
        if let Some(reason) = blocked_path_reason(path) {
            violations.push(GuardViolation {
                path: path.clone(),
                reason,
            });
            continue;
        }
        let output = Command::new("git")
            .args(["show", "--no-textconv", &format!(":{path}")])
            .output()
            .with_context(|| format!("reading staged entry {path}"))?;
        if !output.status.success() {
            bail!(
                "could not inspect staged entry {path}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        violations.extend(inspect_entry(path, &output.stdout));
    }
    Ok(violations)
}

fn blocked_path_reason(path: &str) -> Option<&'static str> {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let file_name = Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if normalized.starts_with("data/hidden/") {
        return Some("hidden evaluation material must remain private");
    }
    if file_name == "sampling-secret.txt" {
        return Some("hidden-set sampling secret must remain private");
    }
    if file_name == ".env" || (file_name.starts_with(".env.") && file_name != ".env.example") {
        return Some("environment file may contain credentials");
    }
    if matches!(file_name, "credentials.json" | "service-account.json") {
        return Some("credential file must not be published");
    }
    if matches!(
        Path::new(file_name)
            .extension()
            .and_then(|value| value.to_str()),
        Some("key" | "pem" | "p12" | "pfx")
    ) {
        return Some("private key or certificate bundle must not be published");
    }
    None
}

fn inspect_entry(path: &str, bytes: &[u8]) -> Vec<GuardViolation> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Vec::new();
    };
    let mut violations = Vec::new();
    let private_key_markers = [
        ["-----BEGIN ", "PRIVATE KEY-----"].concat(),
        ["-----BEGIN RSA ", "PRIVATE KEY-----"].concat(),
        ["-----BEGIN OPENSSH ", "PRIVATE KEY-----"].concat(),
    ];
    if private_key_markers
        .iter()
        .any(|marker| text.contains(marker))
    {
        violations.push(GuardViolation {
            path: path.to_owned(),
            reason: "private key material detected",
        });
    }
    if token_pattern().is_match(text) {
        violations.push(GuardViolation {
            path: path.to_owned(),
            reason: "high-confidence credential token detected",
        });
    }
    for captures in assignment_pattern().captures_iter(text) {
        let value = captures
            .name("value")
            .expect("credential assignment pattern has a value capture")
            .as_str();
        if !looks_like_placeholder(value) {
            violations.push(GuardViolation {
                path: path.to_owned(),
                reason: "credential environment variable contains a literal value",
            });
            break;
        }
    }
    violations
}

fn token_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"(?i)(?:\bAKIA[0-9A-Z]{16}\b|\bAIza[0-9A-Za-z_-]{30,}\b|\bgithub_pat_[0-9A-Za-z_]{20,}\b|\bgh[pousr]_[0-9A-Za-z]{20,}\b|\b(?:sk|xai)-[0-9A-Za-z_-]{20,}\b)",
        )
        .expect("credential token regex is valid")
    })
}

fn assignment_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r#"(?im)^\s*(?:OPENAI_API_KEY|ANTHROPIC_API_KEY|GEMINI_API_KEY|XAI_API_KEY|AWS_SECRET_ACCESS_KEY|GITHUB_TOKEN)\s*=\s*["']?(?<value>[^\s"'#]{12,})"#,
        )
        .expect("credential assignment regex is valid")
    })
}

fn looks_like_placeholder(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    value.starts_with('$')
        || value.starts_with('%')
        || value.starts_with('<')
        || upper.starts_with("YOUR_")
        || upper.starts_with("EXAMPLE")
        || upper.starts_with("REPLACE")
        || upper.starts_with("CHANGEME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_hidden_paths_and_secret_files() {
        assert!(blocked_path_reason("data/hidden/cases.jsonl").is_some());
        assert!(blocked_path_reason("ops/sampling-secret.txt").is_some());
        assert!(blocked_path_reason(".env.production").is_some());
        assert!(blocked_path_reason(".env.example").is_none());
    }

    #[test]
    fn detects_tokens_without_returning_their_values() {
        let fixture = format!(
            "OPENAI_API_KEY={}{}",
            "sk-proj-", "abcdefghijklmnopqrstuvwxyz123456"
        );
        let violations = inspect_entry("accidental.txt", fixture.as_bytes());
        assert!(!violations.is_empty());
        assert!(
            violations
                .iter()
                .all(|violation| !violation.reason.contains("abcdefghijklmnopqrstuvwxyz"))
        );
    }

    #[test]
    fn permits_documented_placeholders() {
        assert!(inspect_entry("README.md", b"OPENAI_API_KEY=$OPENAI_API_KEY").is_empty());
        assert!(inspect_entry(".env.example", b"OPENAI_API_KEY=YOUR_API_KEY_HERE").is_empty());
    }
}
