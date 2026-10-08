use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::selection::{SelectionSpec, select};
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
    /// Maximum number of cases, keeping reference groups complete.
    pub case_limit: Option<usize>,
    /// Optional translation identifier to select before limiting cases.
    pub translation: Option<String>,
    /// Seed for reproducible, stratified reference selection and presentation.
    pub seed: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selection: Option<SelectionSpec>,
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
struct Trial {
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

/// Local trial manager with an operator-selected dataset and output directory.
pub struct McpServer {
    catalog: TranslationCatalog,
    cases: Vec<BenchmarkCase>,
    references: Vec<ReferenceRecord>,
    defaults: McpConfig,
    active: Option<Trial>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BeginTrial {
    run_id: String,
    model: String,
    case_limit: Option<usize>,
    translation: Option<String>,
    seed: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResumeTrial {
    run_id: String,
}

impl McpServer {
    /// Validates the dataset and optionally opens an initial trial.
    /// Empty run and model labels start an idle server.
    ///
    /// # Errors
    /// Rejects invalid datasets, selection settings, labels, or saved progress.
    pub fn open(
        catalog: TranslationCatalog,
        cases: Vec<BenchmarkCase>,
        references: Vec<ReferenceRecord>,
        config: &McpConfig,
    ) -> Result<Self> {
        validate_dataset(&catalog, &cases, &references)?;
        let mut validation = config.clone();
        if config.run_id.is_empty() && config.model.is_empty() {
            validation.run_id = "validation".into();
            validation.model = "validation".into();
        }
        validate_config(&validation)?;
        let active = if config.run_id.is_empty() && config.model.is_empty() {
            None
        } else {
            Some(Trial::open(
                catalog.clone(),
                cases.clone(),
                references.clone(),
                config,
            )?)
        };
        if active.is_none() {
            select(&cases, &selection_spec(config))?;
        }
        Ok(Self {
            catalog,
            cases,
            references,
            defaults: config.clone(),
            active,
        })
    }

    pub(super) fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            "begin_trial" => {
                self.begin(serde_json::from_value(arguments).context("invalid trial arguments")?)
            }
            "resume_trial" => {
                self.resume(serde_json::from_value(arguments).context("invalid resume arguments")?)
            }
            "benchmark_status" if self.active.is_none() && arguments == json!({}) => Ok(json!({
                "active_trial": false, "provider_api_calls": 0, "evidence": "interactive_mcp", "selection_defaults": selection_spec(&self.defaults)
            })),
            _ => self
                .active
                .as_mut()
                .context("no active trial; call begin_trial or resume_trial first")?
                .call_tool(name, arguments),
        }
    }

    fn begin(&mut self, request: BeginTrial) -> Result<Value> {
        let config = McpConfig {
            run_id: request.run_id,
            model: request.model,
            case_limit: request.case_limit.or(self.defaults.case_limit),
            translation: request
                .translation
                .or_else(|| self.defaults.translation.clone()),
            seed: request.seed.unwrap_or_else(|| self.defaults.seed.clone()),
            output_dir: self.defaults.output_dir.clone(),
        };
        validate_config(&config)?;
        if config.model.contains("model unspecified") {
            bail!("provide an explicit model label");
        }
        if let Some(active) = &self.active {
            if active.session.identity.run_id == config.run_id {
                if active.session.identity.model == config.model
                    && active.session.identity.selection.as_ref() == Some(&selection_spec(&config))
                {
                    return Ok(active.status());
                }
                bail!("active trial settings differ; use a new run_id");
            }
        }
        self.check_switch()?;
        if config
            .output_dir
            .join(format!("run-{}/session.json", config.run_id))
            .exists()
        {
            bail!("run_id already exists; call resume_trial");
        }
        self.activate(&config)
    }

    fn resume(&mut self, request: ResumeTrial) -> Result<Value> {
        let mut config = self.defaults.clone();
        config.run_id = request.run_id;
        config.model = "validation".into();
        validate_config(&config)?;
        if let Some(active) = &self.active {
            if active.session.identity.run_id == config.run_id {
                return Ok(active.status());
            }
        }
        self.check_switch()?;
        let saved: Session = read_json(
            &config
                .output_dir
                .join(format!("run-{}/session.json", config.run_id)),
        )?;
        config.model = saved.identity.model;
        if let Some(selection) = saved.identity.selection {
            if selection.method != "stratified_reference_v1" {
                bail!("unsupported selection method");
            }
            config.case_limit = selection.case_limit;
            config.translation = selection.translation;
            config.seed = selection.seed;
        } else {
            let selected: Vec<_> = self
                .cases
                .iter()
                .filter(|case| saved.identity.expected_case_ids.contains(&case.case_id))
                .collect();
            config.case_limit = Some(saved.identity.expected_case_ids.len());
            config.translation = selected
                .first()
                .filter(|first| {
                    selected
                        .iter()
                        .all(|case| case.translation == first.translation)
                })
                .map(|case| case.translation.clone());
        }
        self.activate(&config)
    }

    fn check_switch(&self) -> Result<()> {
        if self.active.as_ref().is_some_and(|trial| {
            !trial.session.finished
                && (trial.session.pending || !trial.session.responses.is_empty())
        }) {
            bail!("finish the active trial before switching runs");
        }
        Ok(())
    }

    fn activate(&mut self, config: &McpConfig) -> Result<Value> {
        let trial = Trial::open(
            self.catalog.clone(),
            self.cases.clone(),
            self.references.clone(),
            config,
        )?;
        let status = trial.status();
        self.active = Some(trial);
        Ok(status)
    }
}

fn selection_spec(config: &McpConfig) -> SelectionSpec {
    SelectionSpec {
        method: "stratified_reference_v1".into(),
        seed: config.seed.clone(),
        case_limit: config.case_limit,
        translation: config.translation.clone(),
    }
}

impl Trial {
    /// Validates a recall dataset and creates or resumes a settings-bound trial.
    ///
    /// # Errors
    /// Rejects unsafe labels, invalid datasets, copy controls, changed resume
    /// inputs, corrupt progress, concurrent writers, or filesystem failures.
    ///
    /// # Panics
    /// Panics if a validated case has no translation, violating dataset invariants.
    fn open(
        catalog: TranslationCatalog,
        cases: Vec<BenchmarkCase>,
        references: Vec<ReferenceRecord>,
        config: &McpConfig,
    ) -> Result<Self> {
        validate_config(config)?;
        validate_dataset(&catalog, &cases, &references)?;
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
        let saved: Option<Session> = path.exists().then(|| read_json(&path)).transpose()?;
        let legacy = saved
            .as_ref()
            .is_some_and(|session| session.identity.schema_version == 1);
        let selection = (!legacy).then(|| SelectionSpec {
            method: "stratified_reference_v1".into(),
            seed: config.seed.clone(),
            case_limit: config.case_limit,
            translation: config.translation.clone(),
        });
        let cases = if let Some(spec) = &selection {
            select(&cases, spec)?
        } else {
            cases
                .into_iter()
                .filter(|case| {
                    config
                        .translation
                        .as_ref()
                        .is_none_or(|id| id == &case.translation)
                })
                .take(config.case_limit.unwrap_or(usize::MAX))
                .collect()
        };
        validate_dataset(&catalog, &cases, &references)?;
        if cases
            .iter()
            .any(|case| case.prompt_variant == PromptVariant::CopyControl)
        {
            bail!("MCP recall trials do not expose copy-control reference text");
        }
        let identity = TrialMetadata {
            schema_version: if legacy { 1 } else { 2 },
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
            selection,
        };
        let session = if let Some(saved) = saved {
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
            "provider_api_calls": 0,
            "active_trial": true,
            "selection": self.session.identity.selection,
            "needs_model_label": self.session.identity.model.contains("model unspecified")
        })
    }

    fn pending_case(&self) -> Option<&BenchmarkCase> {
        self.session
            .pending
            .then(|| self.cases.get(self.session.responses.len()))
            .flatten()
    }

    fn next_case(&mut self) -> Result<Value> {
        if self.session.identity.model.contains("model unspecified") {
            bail!("begin_trial requires an explicit model label before issuing prompts");
        }
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
                "# Interactive MCP trial\n\nEvidence: `interactive_mcp`. Model identity is self-reported. Context isolation, model sampling settings, and absence of retrieval are not verified. Do not treat this as a controlled closed-book provider run.\n\n{}",
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
                "limitations": ["self-reported model identity", "shared conversation context", "client-side retrieval restrictions are not enforced", "model sampling settings and usage are unknown"]
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
