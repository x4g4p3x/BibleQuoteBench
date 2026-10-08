use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    BenchmarkCase, PromptVariant, ReferenceRecord, ResponseRecord, TranslationCatalog,
    aggregate_scores,
    io::{read_json, write_json, write_jsonl, write_text},
    report::{build_report, render_markdown},
    score_response,
    study::digest,
    validate_dataset,
};

/// Operator-selected inputs for one resumable, local interactive trial.
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Stable run identifier, restricted to ASCII letters, digits, `_`, and `-`.
    pub run_id: String,
    /// Self-reported assistant model label; no provider verifies it.
    pub model: String,
    /// Parent directory for `run-<run_id>` artifacts, selected outside MCP tools.
    pub output_dir: PathBuf,
    /// Optional positive prefix size after translation filtering.
    pub case_limit: Option<usize>,
    /// Optional translation identifier to select before limiting cases.
    pub translation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrialMetadata {
    schema_version: u16,
    engine_version: String,
    evidence: String,
    run_id: String,
    model: String,
    model_identity: String,
    cases_sha256: String,
    references_sha256: String,
    catalog_sha256: String,
    prompts_sha256: String,
    expected_case_ids: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Session {
    identity: TrialMetadata,
    responses: Vec<ResponseRecord>,
    pending: bool,
    finished: bool,
}

/// A single dataset-bound trial. Holds an exclusive output lock until dropped.
pub struct McpServer {
    catalog: TranslationCatalog,
    cases: Vec<BenchmarkCase>,
    references: Vec<ReferenceRecord>,
    session: Session,
    output: PathBuf,
    _lock: File,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Answer {
    case_id: String,
    output: String,
}

impl McpServer {
    /// Validates a recall dataset and creates or resumes a settings-bound trial.
    ///
    /// # Errors
    /// Rejects unsafe labels, invalid datasets, copy controls, changed resume
    /// inputs, corrupt progress, concurrent writers, or filesystem failures.
    ///
    /// # Panics
    /// Panics if a validated case has no translation, violating dataset invariants.
    pub fn open(
        catalog: TranslationCatalog,
        mut cases: Vec<BenchmarkCase>,
        references: Vec<ReferenceRecord>,
        config: &McpConfig,
    ) -> Result<Self> {
        validate_config(config)?;
        validate_dataset(&catalog, &cases, &references)?;
        if let Some(translation) = &config.translation {
            cases.retain(|case| &case.translation == translation);
        }
        if let Some(limit) = config.case_limit {
            cases.truncate(limit);
        }
        validate_dataset(&catalog, &cases, &references)?;
        if cases
            .iter()
            .any(|case| case.prompt_variant == PromptVariant::CopyControl)
        {
            bail!("MCP recall trials do not expose copy-control reference text");
        }
        let identity = TrialMetadata {
            schema_version: 1,
            engine_version: env!("CARGO_PKG_VERSION").into(),
            evidence: "interactive_mcp".into(),
            run_id: config.run_id.clone(),
            model: config.model.clone(),
            model_identity: "self_reported".into(),
            cases_sha256: digest(&cases),
            references_sha256: digest(&references),
            catalog_sha256: digest(&catalog),
            prompts_sha256: digest(
                &cases
                    .iter()
                    .map(|case| {
                        let translation = catalog
                            .translations
                            .iter()
                            .find(|spec| spec.id == case.translation)
                            .expect("validated translation");
                        crate::render_prompt(case, translation)
                    })
                    .collect::<Vec<_>>(),
            ),
            expected_case_ids: cases.iter().map(|case| case.case_id.clone()).collect(),
        };
        fs::create_dir_all(&config.output_dir)?;
        let output = config.output_dir.join(format!("run-{}", config.run_id));
        fs::create_dir_all(&output)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(output.join("session.lock"))?;
        lock.try_lock_exclusive()
            .context("another MCP server owns this run")?;
        let path = output.join("session.json");
        let session = if path.exists() {
            let saved: Session = read_json(&path)?;
            if saved.identity != identity {
                bail!("resume settings or dataset changed; use a new run_id");
            }
            validate_progress(&saved, &cases)?;
            saved
        } else {
            Session {
                identity,
                responses: Vec::new(),
                pending: false,
                finished: false,
            }
        };
        let server = Self {
            catalog,
            cases,
            references,
            session,
            output,
            _lock: lock,
        };
        server.persist(&server.session)?;
        Ok(server)
    }

    pub(super) fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        if name == "submit_answer" {
            return self
                .submit(serde_json::from_value(arguments).context("invalid answer arguments")?);
        }
        if arguments != json!({}) {
            bail!("this tool takes no arguments");
        }
        match name {
            "benchmark_status" => Ok(self.status()),
            "next_case" => self.next_case(),
            "finish_run" => self.finish(),
            _ => bail!("unknown tool: {name}"),
        }
    }

    fn status(&self) -> Value {
        json!({
            "run_id": self.session.identity.run_id,
            "model": self.session.identity.model,
            "model_identity": "self_reported",
            "evidence": "interactive_mcp",
            "total_cases": self.cases.len(),
            "answered_cases": self.session.responses.len(),
            "pending_case_id": self.pending_case().map(|case| &case.case_id),
            "finished": self.session.finished,
            "provider_api_calls": 0
        })
    }

    fn pending_case(&self) -> Option<&BenchmarkCase> {
        self.session
            .pending
            .then(|| self.cases.get(self.session.responses.len()))
            .flatten()
    }

    fn next_case(&mut self) -> Result<Value> {
        if self.session.finished || self.session.responses.len() == self.cases.len() {
            return Ok(json!({"complete": true, "progress": self.status()}));
        }
        let mut updated = self.session.clone();
        updated.pending = true;
        self.persist(&updated)?;
        self.session = updated;
        let case = &self.cases[self.session.responses.len()];
        let translation = self
            .catalog
            .translations
            .iter()
            .find(|spec| spec.id == case.translation)
            .expect("validated translation");
        Ok(json!({
            "complete": false,
            "case_id": case.case_id,
            "prompt": crate::render_prompt(case, translation),
            "progress": self.status()
        }))
    }

    fn submit(&mut self, answer: Answer) -> Result<Value> {
        if answer.output.chars().count() > 16_384 {
            bail!("answer exceeds 16384 characters");
        }
        if let Some(previous) = self
            .session
            .responses
            .iter()
            .find(|record| record.case_id == answer.case_id)
        {
            if previous.output == answer.output {
                return Ok(
                    json!({"accepted": true, "already_recorded": true, "progress": self.status()}),
                );
            }
            bail!("answer is immutable; a different answer was already recorded");
        }
        if self
            .pending_case()
            .is_none_or(|case| case.case_id != answer.case_id)
        {
            bail!("submit only the currently issued case; call next_case first");
        }
        let response = ResponseRecord {
            case_id: answer.case_id,
            run_id: self.session.identity.run_id.clone(),
            provider: "interactive_mcp".into(),
            model: self.session.identity.model.clone(),
            resolved_model: None,
            output: answer.output,
            error: None,
            temperature: None,
            reasoning_effort: None,
            seed: None,
            provider_request_id: None,
            system_fingerprint: None,
            execution: None,
        };
        let mut updated = self.session.clone();
        updated.responses.push(response);
        updated.pending = false;
        self.persist(&updated)?;
        self.session = updated;
        Ok(json!({"accepted": true, "already_recorded": false, "progress": self.status()}))
    }

    fn finish(&mut self) -> Result<Value> {
        if self.session.responses.len() != self.cases.len() {
            bail!("finish_run requires an answer for every selected case");
        }
        let scores: Vec<_> = self
            .cases
            .iter()
            .zip(&self.session.responses)
            .map(|(case, response)| {
                let requested = self
                    .references
                    .iter()
                    .find(|record| {
                        record.translation == case.translation && record.reference == case.reference
                    })
                    .expect("validated reference");
                let alternatives: Vec<_> = self
                    .references
                    .iter()
                    .filter(|record| {
                        record.translation != case.translation && record.reference == case.reference
                    })
                    .collect();
                score_response(case, response, requested, &alternatives)
            })
            .collect();
        let report = build_report(&scores);
        write_jsonl(
            Some(&self.output.join("responses.jsonl")),
            &self.session.responses,
        )?;
        write_jsonl(Some(&self.output.join("scores.jsonl")), &scores)?;
        write_json(&self.output.join("report.json"), &report)?;
        write_text(
            &self.output.join("report.md"),
            &format!(
                "# Interactive MCP trial\n\nEvidence: `interactive_mcp`. Model identity is self-reported. Context isolation, sampling settings, and absence of retrieval are not verified. Do not treat this as a controlled closed-book provider run.\n\n{}",
                render_markdown(&report)
            ),
        )?;
        write_json(
            &self.output.join("trial.json"),
            &json!({
                "identity": self.session.identity,
                "responses_sha256": digest(&self.session.responses),
                "scores_sha256": digest(&scores),
                "provider_api_calls": 0,
                "limitations": ["self-reported model identity", "shared conversation context", "client-side retrieval restrictions are not enforced", "sampling settings and usage are unknown"]
            }),
        )?;
        let mut updated = self.session.clone();
        updated.finished = true;
        self.persist(&updated)?;
        self.session = updated;
        Ok(
            json!({"progress": self.status(), "summary": aggregate_scores(&scores), "output_dir": self.output}),
        )
    }

    fn persist(&self, session: &Session) -> Result<()> {
        let temporary = self.output.join("session.checkpoint.tmp");
        let mut file = File::create(&temporary)?;
        serde_json::to_writer(&mut file, session)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, self.output.join("session.json"))?;
        Ok(())
    }
}

fn validate_config(config: &McpConfig) -> Result<()> {
    if config.run_id.is_empty()
        || config.run_id.len() > 64
        || !config
            .run_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        bail!("run_id must contain 1-64 ASCII letters, digits, underscores or hyphens");
    }
    if config.model.trim().is_empty() || config.model.len() > 256 {
        bail!("model must be a nonempty label of at most 256 bytes");
    }
    if config.case_limit == Some(0) {
        bail!("case_limit must be positive");
    }
    Ok(())
}

fn validate_progress(session: &Session, cases: &[BenchmarkCase]) -> Result<()> {
    if session.responses.len() > cases.len()
        || (session.pending && (session.finished || session.responses.len() == cases.len()))
        || (session.finished && session.responses.len() != cases.len())
    {
        bail!("invalid saved MCP progress");
    }
    for (response, case) in session.responses.iter().zip(cases) {
        if response.case_id != case.case_id
            || response.run_id != session.identity.run_id
            || response.model != session.identity.model
            || response.provider != "interactive_mcp"
            || response.resolved_model.is_some()
            || response.error.is_some()
            || response.execution.is_some()
            || response.temperature.is_some()
            || response.reasoning_effort.is_some()
            || response.seed.is_some()
            || response.provider_request_id.is_some()
            || response.system_fingerprint.is_some()
            || response.output.chars().count() > 16_384
        {
            bail!("saved MCP response does not match the trial");
        }
    }
    Ok(())
}
