//! Manifest-bound evaluation with complete coverage and paired reference-level inference.

use crate::{
    BenchmarkCase, Classification, LicenseKind, PromptVariant, ReferenceRecord, ResponseRecord,
    ScoreRecord, TranslationCatalog, aggregate_scores, score_response,
};
use crate::{
    importer::sha256_hex,
    io::{read_json, read_jsonl, write_json, write_jsonl, write_text},
    prompt::execution_prompt,
    provider::ProviderConfig,
    report::{BenchmarkReport, build_report, render_markdown},
    statistics::{Interval, bootstrap},
};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    pub schema_version: u16,
    pub engine_version: String,
    pub run_id: String,
    pub provider: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub max_output_tokens: u32,
    pub endpoint_sha256: String,
    pub cases_sha256: String,
    pub references_sha256: String,
    pub catalog_sha256: String,
    pub prompts_sha256: String,
    pub responses_sha256: String,
    pub expected_case_ids: Vec<String>,
    pub track: String,
    pub evidence: String,
}

/// Canonical JSON hash for typed, order-preserving benchmark records.
///
/// # Panics
/// Panics if a value cannot be represented as JSON.
pub fn digest<T: Serialize + ?Sized>(value: &T) -> String {
    sha256_hex(&serde_json::to_vec(value).expect("benchmark values serialize"))
}

/// Path of the manifest accompanying a response file.
pub fn manifest_path(responses: &Path) -> PathBuf {
    responses.with_extension("manifest.json")
}

/// Binds a run to the complete intended dataset, prompts, settings, and responses.
///
/// # Errors
/// Rejects invalid datasets or mixtures of diagnostic tracks.
pub fn make_manifest(
    config: &ProviderConfig,
    cases: &[BenchmarkCase],
    references: &[ReferenceRecord],
    catalog: &TranslationCatalog,
    responses: &[ResponseRecord],
) -> Result<RunManifest> {
    crate::validate_dataset(catalog, cases, references)?;
    let track = track(cases)?;
    Ok(RunManifest {
        schema_version: 2,
        engine_version: env!("CARGO_PKG_VERSION").into(),
        run_id: config.run_id.clone(),
        provider: config.kind.name().into(),
        model: config.model.clone(),
        temperature: config.temperature,
        reasoning_effort: config.reasoning_effort.clone(),
        max_output_tokens: config.max_output_tokens,
        endpoint_sha256: sha256_hex(
            config
                .base_url
                .as_deref()
                .unwrap_or(config.kind.default_base_url())
                .as_bytes(),
        ),
        cases_sha256: digest(cases),
        references_sha256: digest(references),
        catalog_sha256: digest(catalog),
        prompts_sha256: prompt_digest(cases, references, catalog)?,
        responses_sha256: digest(responses),
        expected_case_ids: cases.iter().map(|case| case.case_id.clone()).collect(),
        track,
        evidence: "live_provider".into(),
    })
}

fn track(cases: &[BenchmarkCase]) -> Result<String> {
    let tracks: BTreeSet<_> = cases
        .iter()
        .map(|case| {
            if case.reference.verse_end.is_some() {
                "passage".to_owned()
            } else {
                serde_json::to_value(case.prompt_variant)
                    .expect("enum")
                    .as_str()
                    .expect("string")
                    .to_owned()
            }
        })
        .collect();
    if tracks.len() != 1 {
        bail!("analyze each prompt/control/passage track separately");
    }
    if cases.iter().any(|case| {
        case.reference.verse_end.is_some() && case.prompt_variant != PromptVariant::Canonical
    }) {
        bail!("passage track requires canonical prompts");
    }
    Ok(tracks.into_iter().next().expect("one track"))
}

fn prompt_digest(
    cases: &[BenchmarkCase],
    references: &[ReferenceRecord],
    catalog: &TranslationCatalog,
) -> Result<String> {
    let prompts: Result<Vec<_>> = cases
        .iter()
        .map(|case| {
            let translation = catalog
                .translations
                .iter()
                .find(|spec| spec.id == case.translation)
                .expect("validated catalog");
            let supplied = references
                .iter()
                .find(|record| {
                    record.translation == case.translation && record.reference == case.reference
                })
                .map(|record| record.text.as_str());
            execution_prompt(case, translation, supplied)
        })
        .collect();
    Ok(digest(&prompts?))
}

/// Usage reported by the provider and conservative campaign cost accounting.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub known_usage_responses: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub accounted_nanoeur: u64,
    pub accounted_responses: usize,
    pub uncertain_billing_responses: usize,
}

fn execution_summary(responses: &[ResponseRecord]) -> Option<ExecutionSummary> {
    let mut summary = ExecutionSummary::default();
    let mut present = false;
    for meta in responses.iter().filter_map(|r| r.execution.as_ref()) {
        present = true;
        if let Some((input, output)) = meta.input_tokens.zip(meta.output_tokens) {
            summary.known_usage_responses += 1;
            summary.input_tokens = summary.input_tokens.saturating_add(input);
            summary.output_tokens = summary.output_tokens.saturating_add(output);
        }
        summary.accounted_responses += usize::from(meta.accounted_nanoeur.is_some());
        summary.accounted_nanoeur = summary
            .accounted_nanoeur
            .saturating_add(meta.accounted_nanoeur.unwrap_or(0));
        summary.uncertain_billing_responses += usize::from(meta.reservation_retained);
    }
    present.then_some(summary)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelAnalysis {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionSummary>,
    pub repetitions: usize,
    pub completed_cases_per_run: usize,
    pub exact_text: Interval,
    pub exact_words: Interval,
    pub provider_error_rate: f64,
    pub recall_given_provider_success: Option<f64>,
    pub differing_editions: BenchmarkReport,
    pub by_translation_and_stratum: BTreeMap<String, BenchmarkReport>,
    pub report: BenchmarkReport,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PairComparison {
    pub left: String,
    pub right: String,
    pub exact_text_difference: Option<Interval>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Example {
    pub model: String,
    pub run_id: String,
    pub case_id: String,
    pub reference: String,
    pub requested_translation: String,
    pub classification: Classification,
    pub diagnostic: String,
    /// Scorer-derived matrix destination; absent in older v0.2 exports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resembles: Option<String>,
    pub expected: String,
    pub output: String,
    pub word_diff: String,
    pub exact_alternative_editions: Vec<String>,
    pub closer_alternative: Option<String>,
    pub closer_alternatives: Vec<String>,
    pub alternative_edition_token_overlap: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StudyReport {
    pub schema_version: u16,
    pub track: String,
    pub evidence: String,
    pub dataset_sha256: String,
    pub notes: Vec<String>,
    pub models: BTreeMap<String, ModelAnalysis>,
    pub comparisons: Vec<PairComparison>,
    pub examples: Vec<Example>,
}

struct Observations {
    manifest: RunManifest,
    runs: BTreeSet<String>,
    scores: Vec<ScoreRecord>,
    responses: Vec<ResponseRecord>,
}

/// Validates coverage and provenance before producing any model comparison.
///
/// # Errors
/// Rejects altered artifacts, incomplete/duplicate runs, inconsistent model metadata,
/// unequal repetitions, mixed evidence, or overlapping reference clusters.
///
/// # Panics
/// Internal lookups assert invariants established by dataset and run validation.
#[allow(clippy::too_many_lines)]
pub fn analyze(
    cases: &[BenchmarkCase],
    references: &[ReferenceRecord],
    catalog: &TranslationCatalog,
    inputs: &[(RunManifest, Vec<ResponseRecord>)],
    resamples: usize,
) -> Result<StudyReport> {
    crate::validate_dataset(catalog, cases, references)?;
    validate_balanced_cases(cases, catalog)?;
    if inputs.is_empty() || !(100..=100_000).contains(&resamples) {
        bail!("provide runs and 100..100000 bootstrap resamples");
    }
    reject_overlaps(cases)?;
    let mut models: BTreeMap<String, Observations> = BTreeMap::new();
    let mut all_runs = BTreeSet::new();
    let mut evidence = BTreeSet::new();
    for (manifest, responses) in inputs {
        validate_run(manifest, responses, cases, references, catalog)?;
        if !all_runs.insert((&manifest.provider, &manifest.model, &manifest.run_id)) {
            bail!("duplicate run identity");
        }
        evidence.insert(manifest.evidence.clone());
        let identity = model_identity(manifest, responses)?;
        let observations = models.entry(identity).or_insert_with(|| Observations {
            manifest: manifest.clone(),
            runs: BTreeSet::new(),
            scores: vec![],
            responses: vec![],
        });
        observations.runs.insert(manifest.run_id.clone());
        for response in responses {
            let case = cases
                .iter()
                .find(|case| case.case_id == response.case_id)
                .expect("validated coverage");
            let requested = references
                .iter()
                .find(|record| {
                    record.translation == case.translation && record.reference == case.reference
                })
                .expect("validated reference");
            let alternatives: Vec<_> = references
                .iter()
                .filter(|record| {
                    record.translation != case.translation && record.reference == case.reference
                })
                .collect();
            observations
                .scores
                .push(score_response(case, response, requested, &alternatives));
            observations.responses.push(response.clone());
        }
    }
    if evidence.len() != 1 {
        bail!("synthetic and live evidence must be analyzed separately");
    }
    let repetition_counts: BTreeSet<_> = models.values().map(|model| model.runs.len()).collect();
    if repetition_counts.len() != 1 {
        bail!("unequal repetitions across model configurations");
    }
    let mut analyses = BTreeMap::new();
    let mut examples = Vec::new();
    for (name, observations) in &models {
        let scores = &observations.scores;
        let success: Vec<_> = scores
            .iter()
            .filter(|score| score.classification != Classification::ProviderError)
            .cloned()
            .collect();
        let differing: Vec<_> = scores
            .iter()
            .filter(|score| {
                let texts: BTreeSet<_> = references
                    .iter()
                    .filter(|record| record.reference == score.reference)
                    .map(|record| &record.text)
                    .collect();
                texts.len() > 1
            })
            .cloned()
            .collect();
        let mut subgroups: BTreeMap<String, Vec<ScoreRecord>> = BTreeMap::new();
        for score in scores {
            subgroups
                .entry(format!(
                    "{} / {}",
                    score.requested_translation,
                    stratum(score)
                ))
                .or_default()
                .push(score.clone());
        }
        analyses.insert(
            name.clone(),
            ModelAnalysis {
                execution: execution_summary(&observations.responses),
                repetitions: observations.runs.len(),
                completed_cases_per_run: cases.len(),
                exact_text: bootstrap(&cluster_values(scores, false), resamples),
                exact_words: bootstrap(&cluster_values(scores, true), resamples),
                provider_error_rate: aggregate_scores(scores).provider_error_rate,
                recall_given_provider_success: (!success.is_empty())
                    .then(|| aggregate_scores(&success).exact_text_rate),
                differing_editions: build_report(&differing),
                by_translation_and_stratum: subgroups
                    .into_iter()
                    .map(|(key, values)| (key, build_report(&values)))
                    .collect(),
                report: build_report(scores),
            },
        );
        examples.extend(failure_examples(name, observations, references, catalog));
    }
    let entries: Vec<_> = models.iter().collect();
    let mut comparisons = Vec::new();
    for (index, (left_name, left)) in entries.iter().enumerate() {
        for (right_name, right) in entries.iter().skip(index + 1) {
            let reason =
                if left.manifest.temperature.is_none() || right.manifest.temperature.is_none() {
                    Some(
                        "Unspecified provider defaults: effective sampling settings are unknown."
                            .into(),
                    )
                } else if left.manifest.reasoning_effort != right.manifest.reasoning_effort
                    || left.manifest.temperature != right.manifest.temperature
                    || left.manifest.max_output_tokens != right.manifest.max_output_tokens
                {
                    Some("Different requested generation settings.".into())
                } else {
                    None
                };
            let difference = reason
                .is_none()
                .then(|| bootstrap(&paired_values(&left.scores, &right.scores), resamples));
            comparisons.push(PairComparison {
                left: (*left_name).clone(),
                right: (*right_name).clone(),
                exact_text_difference: difference,
                reason,
            });
        }
    }
    Ok(StudyReport { schema_version: 2, track: track(cases)?, evidence: evidence.into_iter().next().expect("one evidence kind"), dataset_sha256: digest(cases),
        notes: vec![
            "95% percentile intervals use deterministic stratified reference-cluster bootstrap; translations and repetitions stay together. Intervals are conditional on these runs, not estimates of future provider drift.".into(),
            "Pairwise intervals are exploratory and unadjusted for multiple comparisons. Small pilot strata can yield degenerate intervals; no superiority claim follows automatically.".into(),
            "The overall score is the fixed benchmark mixture, not a population estimate for the whole Bible. Provider errors count as end-to-end failures and are excluded only from conditional recall.".into(),
            "Exact alternative matches establish textual resemblance, not training provenance. Hidden references are not necessarily unseen training text.".into(),
            "Matched requested temperature and token limits do not guarantee identical effective behavior across providers. A missing resolved model identifier is explicitly marked unresolved.".into(),
        ], models: analyses, comparisons, examples })
}

fn validate_run(
    manifest: &RunManifest,
    responses: &[ResponseRecord],
    cases: &[BenchmarkCase],
    references: &[ReferenceRecord],
    catalog: &TranslationCatalog,
) -> Result<()> {
    if manifest.schema_version != 2 || manifest.engine_version != env!("CARGO_PKG_VERSION") {
        bail!("unsupported manifest/engine version");
    }
    if !["live_provider", "synthetic_fixture"].contains(&manifest.evidence.as_str()) {
        bail!("unknown evidence kind");
    }
    if manifest.cases_sha256 != digest(cases)
        || manifest.references_sha256 != digest(references)
        || manifest.catalog_sha256 != digest(catalog)
        || manifest.prompts_sha256 != prompt_digest(cases, references, catalog)?
        || manifest.responses_sha256 != digest(responses)
        || manifest.track != track(cases)?
    {
        bail!("manifest digest/track mismatch: artifacts or prompt contract changed");
    }
    let expected: BTreeSet<_> = cases.iter().map(|case| case.case_id.as_str()).collect();
    let declared: BTreeSet<_> = manifest
        .expected_case_ids
        .iter()
        .map(String::as_str)
        .collect();
    let observed: BTreeSet<_> = responses
        .iter()
        .map(|response| response.case_id.as_str())
        .collect();
    if expected != declared
        || declared.len() != manifest.expected_case_ids.len()
        || expected != observed
        || observed.len() != responses.len()
    {
        bail!(
            "incomplete, duplicate, or unexpected cases: expected {}, observed {}",
            expected.len(),
            responses.len()
        );
    }
    if manifest.run_id.trim().is_empty()
        || manifest.model.trim().is_empty()
        || manifest.max_output_tokens == 0
    {
        bail!("invalid run configuration");
    }
    for response in responses {
        if response.run_id != manifest.run_id
            || response.provider != manifest.provider
            || response.model != manifest.model
            || response.temperature != manifest.temperature
            || response.reasoning_effort != manifest.reasoning_effort
            || response.seed.is_some()
        {
            bail!("response identity/settings disagree with manifest");
        }
        if response.error.is_some() && !response.output.is_empty() {
            bail!("provider-error response must not contain scored output");
        }
    }
    Ok(())
}

fn model_identity(manifest: &RunManifest, responses: &[ResponseRecord]) -> Result<String> {
    let resolved: BTreeSet<_> = responses
        .iter()
        .filter(|response| response.error.is_none())
        .map(|response| {
            (
                response.resolved_model.clone(),
                response.system_fingerprint.clone(),
            )
        })
        .collect();
    if resolved.len() > 1 {
        bail!("resolved model/fingerprint changed within a run");
    }
    let version = resolved.iter().next().cloned().unwrap_or_default();
    let configuration = digest(&(
        manifest.temperature,
        &manifest.reasoning_effort,
        manifest.max_output_tokens,
        &manifest.endpoint_sha256,
        &version,
    ));
    Ok(format!(
        "{} / {} / {} / {}",
        manifest.provider,
        manifest.model,
        version.0.as_deref().unwrap_or("unresolved"),
        &configuration[..12]
    ))
}

fn reject_overlaps(cases: &[BenchmarkCase]) -> Result<()> {
    let mut seen = BTreeMap::new();
    for case in cases {
        for verse in case.reference.verse_start..=case.reference.end_verse() {
            let key = (&case.reference.book, case.reference.chapter, verse);
            if let Some(previous) = seen.insert(key, &case.reference) {
                if previous != &case.reference {
                    bail!("overlapping passages are not independent reference clusters");
                }
            }
        }
    }
    Ok(())
}

fn validate_balanced_cases(cases: &[BenchmarkCase], catalog: &TranslationCatalog) -> Result<()> {
    let expected: BTreeSet<_> = catalog.translations.iter().map(|spec| &spec.id).collect();
    let mut references: BTreeMap<String, Vec<&BenchmarkCase>> = BTreeMap::new();
    for case in cases {
        references
            .entry(case.reference.to_string())
            .or_default()
            .push(case);
    }
    for group in references.values() {
        let translations: BTreeSet<_> = group.iter().map(|case| &case.translation).collect();
        if translations != expected || group.iter().any(|case| case.stratum != group[0].stratum) {
            bail!("each reference requires all catalogued editions in one consistent stratum");
        }
    }
    Ok(())
}

fn stratum(score: &ScoreRecord) -> String {
    serde_json::to_value(score.stratum)
        .expect("enum")
        .as_str()
        .expect("string")
        .into()
}

#[allow(clippy::cast_precision_loss)]
fn reference_means(scores: &[ScoreRecord], words: bool) -> BTreeMap<(String, String), f64> {
    let mut values: BTreeMap<(String, String), Vec<f64>> = BTreeMap::new();
    for score in scores {
        values
            .entry((stratum(score), score.reference.to_string()))
            .or_default()
            .push(f64::from(if words {
                score.exact_words
            } else {
                score.exact_text
            }));
    }
    values
        .into_iter()
        .map(|(key, values)| (key, values.iter().sum::<f64>() / values.len() as f64))
        .collect()
}

fn cluster_values(scores: &[ScoreRecord], words: bool) -> BTreeMap<String, Vec<f64>> {
    let mut groups: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for ((stratum, _), value) in reference_means(scores, words) {
        groups.entry(stratum).or_default().push(value);
    }
    groups
}

fn paired_values(left: &[ScoreRecord], right: &[ScoreRecord]) -> BTreeMap<String, Vec<f64>> {
    let right = reference_means(right, false);
    let mut groups: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for (key, value) in reference_means(left, false) {
        groups
            .entry(key.0.clone())
            .or_default()
            .push(value - right[&key]);
    }
    groups
}

fn failure_examples(
    name: &str,
    observations: &Observations,
    references: &[ReferenceRecord],
    catalog: &TranslationCatalog,
) -> Vec<Example> {
    let public = catalog.translations.iter().all(|spec| {
        spec.license_kind == LicenseKind::PublicDomain && spec.redistribute_reference_text
    }) && observations
        .scores
        .iter()
        .all(|score| !score.case_id.starts_with("BQ-HID-"));
    if !public {
        return vec![];
    }
    let mut selected: BTreeSet<_> = BTreeSet::new();
    let mut pairs: Vec<_> = observations
        .scores
        .iter()
        .zip(&observations.responses)
        .collect();
    pairs.sort_by_key(|(score, _)| (&score.case_id, &score.run_id));
    pairs
        .into_iter()
        .filter(|(score, _)| !score.exact_text && selected.insert(example_kind(score)))
        .take(7)
        .map(|(score, response)| {
            let expected = references
                .iter()
                .find(|record| {
                    record.translation == score.requested_translation
                        && record.reference == score.reference
                })
                .expect("reference")
                .text
                .clone();
            Example {
                model: name.into(),
                run_id: score.run_id.clone(),
                case_id: score.case_id.clone(),
                reference: score.reference.to_string(),
                requested_translation: score.requested_translation.clone(),
                classification: score.classification,
                diagnostic: example_kind(score),
                resembles: Some(crate::report::resemblance(score).into()),
                word_diff: word_diff(&expected, &response.output),
                expected,
                output: response.output.clone(),
                exact_alternative_editions: score.exact_other_translations.clone(),
                closer_alternative: score.closest_translation.clone(),
                closer_alternatives: score.closest_translations.clone(),
                alternative_edition_token_overlap: score.translation_contamination_rate,
            }
        })
        .collect()
}

fn example_kind(score: &ScoreRecord) -> String {
    if score.exact_words {
        "formatting_only".into()
    } else if score.classification == Classification::Partial
        && score
            .translation_contamination_rate
            .is_some_and(|overlap| overlap > 0.0)
    {
        "partial_with_alternative_overlap".into()
    } else {
        format!("{:?}", score.classification)
    }
}

/// Annotates whitespace-token edits: deletions `[-word]`, additions `[+word]`.
pub fn word_diff(expected: &str, output: &str) -> String {
    let left: Vec<_> = expected.split_whitespace().collect();
    let right: Vec<_> = output.split_whitespace().collect();
    let mut distance = vec![vec![0; right.len() + 1]; left.len() + 1];
    for (i, row) in distance.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, value) in distance[0].iter_mut().enumerate() {
        *value = j;
    }
    for i in 1..=left.len() {
        for j in 1..=right.len() {
            distance[i][j] = (distance[i - 1][j - 1] + usize::from(left[i - 1] != right[j - 1]))
                .min(distance[i - 1][j] + 1)
                .min(distance[i][j - 1] + 1);
        }
    }
    let (mut i, mut j) = (left.len(), right.len());
    let mut edits = Vec::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && left[i - 1] == right[j - 1] {
            edits.push(left[i - 1].to_owned());
            i -= 1;
            j -= 1;
        } else if i > 0 && j > 0 && distance[i][j] == distance[i - 1][j - 1] + 1 {
            edits.push(format!("[-{}][+{}]", left[i - 1], right[j - 1]));
            i -= 1;
            j -= 1;
        } else if i > 0 && distance[i][j] == distance[i - 1][j] + 1 {
            edits.push(format!("[-{}]", left[i - 1]));
            i -= 1;
        } else {
            edits.push(format!("[+{}]", right[j - 1]));
            j -= 1;
        }
    }
    edits.reverse();
    edits.join(" ")
}

/// Loads paired response/manifest files and writes a validated analysis.
///
/// # Errors
/// Returns artifact, validation, or output errors.
pub fn analyze_files(
    cases: &[BenchmarkCase],
    references: &[ReferenceRecord],
    catalog: &TranslationCatalog,
    paths: &[PathBuf],
    output: &Path,
    resamples: usize,
) -> Result<()> {
    let inputs: Result<Vec<_>> = paths
        .iter()
        .map(|path| Ok((read_json(&manifest_path(path))?, read_jsonl(path)?)))
        .collect();
    let report = analyze(cases, references, catalog, &inputs?, resamples)?;
    write_study(output, &report)
}

/// Writes JSON, Markdown, and independently reusable annotated examples.
///
/// # Errors
/// Returns output creation/serialization errors.
pub fn write_study(output: &Path, report: &StudyReport) -> Result<()> {
    std::fs::create_dir_all(output)?;
    write_json(&output.join("analysis.json"), report)?;
    write_jsonl(Some(&output.join("examples.jsonl")), &report.examples)?;
    write_text(&output.join("analysis.md"), &study_markdown(report))?;
    crate::visualization::write_html(&output.join("analysis.html"), std::slice::from_ref(report))
}

fn study_markdown(report: &StudyReport) -> String {
    let mut text = format!(
        "# BibleQuoteBench validated analysis\n\nEvidence: **{}**. Track: **{}**.\n\n",
        report.evidence, report.track
    );
    for note in &report.notes {
        let _ = writeln!(text, "- {note}");
    }
    for (name, model) in &report.models {
        let _ = writeln!(
            text,
            "\n## {}\n\n{} complete repetitions; {} cases each; {} reference clusters.\n\nExactText: {}. ExactWords: {}.\n\nProvider errors: {:.2}%. Recall given provider success: {}.\n",
            escape(name),
            model.repetitions,
            model.completed_cases_per_run,
            model.exact_text.clusters,
            interval_text(&model.exact_text),
            interval_text(&model.exact_words),
            100.0 * model.provider_error_rate,
            model.recall_given_provider_success.map_or_else(
                || "undefined (no successful requests)".into(),
                |value| format!("{:.2}%", value * 100.0)
            )
        );
        let breakdown = render_markdown(&model.report)
            .replace("# BibleQuoteBench report", "### Descriptive breakdown")
            .replace("Exploratory descriptive report: supplied rows only; coverage and configuration comparability are not verified here. Use `analyze` for validated comparisons.\n\n", "")
            .replace("\n## ", "\n### ");
        text.push_str(&breakdown);
        let _ = writeln!(
            text,
            "\nEdition-differing cases: {}; ExactText {:.2}%.\n",
            model.differing_editions.overall.responses,
            model.differing_editions.overall.exact_text_rate * 100.0
        );
        text.push_str("| Translation / stratum | Responses | ExactText |\n| --- | ---: | ---: |\n");
        for (group, summary) in &model.by_translation_and_stratum {
            let _ = writeln!(
                text,
                "| {} | {} | {:.2}% |",
                escape(group),
                summary.overall.responses,
                100.0 * summary.overall.exact_text_rate
            );
        }
    }
    text.push_str(
        "\n## Paired comparisons\n\nDifferences are left minus right, in percentage points.\n",
    );
    for pair in &report.comparisons {
        let value = pair
            .exact_text_difference
            .as_ref()
            .map_or_else(|| pair.reason.clone().unwrap_or_default(), interval_text);
        let _ = writeln!(
            text,
            "\n- {} versus {}: {value}",
            escape(&pair.left),
            escape(&pair.right)
        );
    }
    text.push_str("\n## Annotated failures\n\nWhitespace-token display diff; scoring uses the normative Unicode tokenizer. Deletions use [-word], additions [+word]. Examples select the first case per diagnostic category deterministically; they are illustrative, not prevalence estimates.\n");
    for example in &report.examples {
        let _ = writeln!(
            text,
            "\n### {} — {} ({:?})\n\nModel: {}. Requested edition: {}.\n\nExpected: {}\n\nProduced: {}\n\nEdits: {}\n\nExact alternative editions: {}. Closest alternatives: {}. Alternative-edition token overlap: {}.\n",
            escape(&example.reference),
            escape(&example.case_id),
            example.classification,
            escape(&example.model),
            escape(&example.requested_translation),
            escape(&example.expected),
            escape(&example.output),
            escape(&example.word_diff),
            escape(&example.exact_alternative_editions.join(", ")),
            escape(&example.closer_alternatives.join(", ")),
            example.alternative_edition_token_overlap.map_or_else(
                || "undefined".into(),
                |value| format!("{:.2}%", value * 100.0)
            )
        );
    }
    format!(
        "{}\n",
        text.lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
    )
}

fn interval_text(value: &Interval) -> String {
    format!(
        "{:.2}% [95% CI {:.2}, {:.2}]",
        value.estimate * 100.0,
        value.lower * 100.0,
        value.upper * 100.0
    )
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "&#124;")
        .replace('*', "&#42;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
        .replace('`', "&#96;")
        .replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BibleReference, CaseStratum, TranslationSpec, provider::ProviderKind};

    fn fixture() -> (
        Vec<BenchmarkCase>,
        Vec<ReferenceRecord>,
        TranslationCatalog,
        ProviderConfig,
        Vec<ResponseRecord>,
    ) {
        let catalog = TranslationCatalog {
            schema_version: 1,
            translations: ["a", "b"]
                .into_iter()
                .map(|id| TranslationSpec {
                    id: id.into(),
                    name: id.into(),
                    abbreviation: id.into(),
                    edition: "1".into(),
                    license_kind: LicenseKind::PublicDomain,
                    license_url: "https://example.test/license".into(),
                    source_url: "https://example.test/text".into(),
                    redistribute_reference_text: true,
                })
                .collect(),
        };
        let config = ProviderConfig {
            kind: ProviderKind::OpenaiCompatible,
            model: "fixture".into(),
            run_id: "run-1".into(),
            api_key_env: None,
            base_url: None,
            temperature: Some(0.0),
            reasoning_effort: None,
            max_output_tokens: 512,
            case_limit: None,
            fail_fast: false,
        };
        let mut cases = Vec::new();
        let mut references = Vec::new();
        let mut responses = Vec::new();
        for verse in 1..=4 {
            for translation in ["a", "b"] {
                let reference = BibleReference {
                    book: "John".into(),
                    chapter: 1,
                    verse_start: verse,
                    verse_end: None,
                };
                let case_id = format!("BQ-DEV-{verse}-{translation}");
                cases.push(BenchmarkCase {
                    case_id: case_id.clone(),
                    translation: translation.into(),
                    reference: reference.clone(),
                    stratum: CaseStratum::Random,
                    prompt_variant: PromptVariant::Canonical,
                });
                let text = format!("The {translation} verse {verse}.");
                references.push(ReferenceRecord {
                    translation: translation.into(),
                    reference,
                    text: text.clone(),
                });
                responses.push(ResponseRecord {
                    case_id,
                    run_id: config.run_id.clone(),
                    provider: config.kind.name().into(),
                    model: config.model.clone(),
                    resolved_model: Some("fixture-version".into()),
                    output: text,
                    error: None,
                    temperature: config.temperature,
                    reasoning_effort: config.reasoning_effort.clone(),
                    seed: None,
                    provider_request_id: None,
                    system_fingerprint: None,
                    execution: None,
                });
            }
        }
        (cases, references, catalog, config, responses)
    }

    fn input(
        config: &ProviderConfig,
        cases: &[BenchmarkCase],
        references: &[ReferenceRecord],
        catalog: &TranslationCatalog,
        responses: Vec<ResponseRecord>,
    ) -> (RunManifest, Vec<ResponseRecord>) {
        (
            make_manifest(config, cases, references, catalog, &responses).unwrap(),
            responses,
        )
    }

    #[test]
    fn paired_analysis_keeps_translations_and_repetitions_in_four_clusters() {
        let (cases, references, catalog, config, responses) = fixture();
        let first = input(&config, &cases, &references, &catalog, responses.clone());
        let mut other_config = config.clone();
        other_config.model = "other".into();
        let mut other = responses;
        for (index, response) in other.iter_mut().enumerate() {
            response.model = "other".into();
            if index < 4 {
                response.output = "wrong".into();
            }
        }
        let second = input(&other_config, &cases, &references, &catalog, other);
        let report = analyze(&cases, &references, &catalog, &[first, second], 200).unwrap();
        assert_eq!(report.models.len(), 2);
        assert_eq!(
            report.models.values().next().unwrap().exact_text.clusters,
            4
        );
        let delta = report.comparisons[0]
            .exact_text_difference
            .as_ref()
            .unwrap();
        assert!((delta.estimate - 0.5).abs() < f64::EPSILON);
        assert_eq!(delta.clusters, 4);
        assert!(delta.lower < delta.upper);
        assert!(
            report
                .models
                .values()
                .all(|model| model.differing_editions.overall.responses == 8)
        );
        assert_eq!(
            report
                .models
                .values()
                .next()
                .unwrap()
                .by_translation_and_stratum
                .len(),
            2
        );
        assert!(study_markdown(&report).contains("Paired comparisons"));
    }

    #[test]
    fn rejects_tampering_missing_duplicate_and_unexpected_cases() {
        let (cases, references, catalog, config, responses) = fixture();
        let pristine = input(&config, &cases, &references, &catalog, responses);
        let mut changed = pristine.clone();
        changed.1[0].output = "tampered".into();
        assert!(
            analyze(&cases, &references, &catalog, &[changed], 100)
                .unwrap_err()
                .to_string()
                .contains("digest")
        );
        for modification in 0..4 {
            let mut changed = pristine.clone();
            match modification {
                0 => {
                    changed.1.pop();
                }
                1 => changed.1.push(changed.1[0].clone()),
                2 => changed.1[0].case_id = "unexpected".into(),
                _ => changed.0.expected_case_ids.push("unexpected".into()),
            }
            changed.0.responses_sha256 = digest(&changed.1);
            assert!(
                analyze(&cases, &references, &catalog, &[changed], 100)
                    .unwrap_err()
                    .to_string()
                    .contains("cases")
            );
        }
        let mut changed = pristine.clone();
        changed.0.catalog_sha256 = "wrong".into();
        assert!(analyze(&cases, &references, &catalog, &[changed], 100).is_err());
        let mut changed = pristine;
        changed.0.schema_version = 999;
        assert!(analyze(&cases, &references, &catalog, &[changed], 100).is_err());
    }

    #[test]
    fn rejects_configuration_and_version_drift() {
        let (cases, references, catalog, config, responses) = fixture();
        for modification in 0..7 {
            let mut changed = responses.clone();
            match modification {
                0 => changed[0].temperature = Some(1.0),
                1 => changed[0].run_id = "other".into(),
                2 => changed[0].provider = "other".into(),
                3 => changed[0].model = "other".into(),
                4 => changed[0].seed = Some(1),
                5 => changed[0].resolved_model = Some("drift".into()),
                _ => changed[0].system_fingerprint = Some("drift".into()),
            }
            let input = input(&config, &cases, &references, &catalog, changed);
            assert!(analyze(&cases, &references, &catalog, &[input], 100).is_err());
        }
    }

    #[test]
    fn requires_equal_repetitions_and_unique_runs() {
        let (cases, references, catalog, config, responses) = fixture();
        let first = input(&config, &cases, &references, &catalog, responses.clone());
        assert!(
            analyze(
                &cases,
                &references,
                &catalog,
                &[first.clone(), first.clone()],
                100
            )
            .unwrap_err()
            .to_string()
            .contains("duplicate run")
        );
        let mut second_config = config.clone();
        second_config.run_id = "run-2".into();
        let mut second_responses = responses.clone();
        for response in &mut second_responses {
            response.run_id = "run-2".into();
        }
        let second = input(
            &second_config,
            &cases,
            &references,
            &catalog,
            second_responses,
        );
        let mut other_config = config;
        other_config.model = "other".into();
        let mut other = responses;
        for response in &mut other {
            response.model = "other".into();
        }
        let third = input(&other_config, &cases, &references, &catalog, other);
        assert!(
            analyze(
                &cases,
                &references,
                &catalog,
                &[first.clone(), second.clone(), third],
                100
            )
            .unwrap_err()
            .to_string()
            .contains("unequal repetitions")
        );
        let report = analyze(&cases, &references, &catalog, &[first, second], 100).unwrap();
        let model = report.models.values().next().unwrap();
        assert_eq!(model.repetitions, 2);
        assert_eq!(model.exact_text.clusters, 4);
        assert_eq!(
            model
                .report
                .stability
                .values()
                .next()
                .unwrap()
                .repeated_cases,
            8
        );
    }

    #[test]
    fn mismatched_or_unknown_settings_are_descriptive_only() {
        let (cases, references, catalog, config, responses) = fixture();
        let first = input(&config, &cases, &references, &catalog, responses.clone());
        for temperature in [None, Some(1.0)] {
            let mut other_config = config.clone();
            other_config.model = "other".into();
            other_config.temperature = temperature;
            let mut other = responses.clone();
            for response in &mut other {
                response.model = "other".into();
                response.temperature = temperature;
            }
            let second = input(&other_config, &cases, &references, &catalog, other);
            let report =
                analyze(&cases, &references, &catalog, &[first.clone(), second], 100).unwrap();
            assert!(report.comparisons[0].exact_text_difference.is_none());
            assert!(report.comparisons[0].reason.is_some());
        }
    }

    #[test]
    fn provider_errors_have_separate_denominator_and_no_false_stability() {
        let (cases, references, catalog, config, mut responses) = fixture();
        responses[0].error = Some("failed".into());
        responses[0].output.clear();
        let first = input(&config, &cases, &references, &catalog, responses.clone());
        let report = analyze(&cases, &references, &catalog, &[first], 100).unwrap();
        let model = report.models.values().next().unwrap();
        assert!((model.exact_text.estimate - 0.875).abs() < f64::EPSILON);
        assert!((model.recall_given_provider_success.unwrap() - 1.0).abs() < f64::EPSILON);
        assert!((model.provider_error_rate - 0.125).abs() < f64::EPSILON);
        responses[0].output = "must not be counted".into();
        let invalid = input(&config, &cases, &references, &catalog, responses.clone());
        assert!(analyze(&cases, &references, &catalog, &[invalid], 100).is_err());
        for response in &mut responses {
            response.output.clear();
            response.error = Some("failed".into());
        }
        let failed = input(&config, &cases, &references, &catalog, responses);
        let report = analyze(&cases, &references, &catalog, &[failed], 100).unwrap();
        let model = report.models.values().next().unwrap();
        assert!(model.recall_given_provider_success.is_none());
        assert!(model.report.stability.is_empty());
        assert!(study_markdown(&report).contains("undefined (no successful requests)"));
    }

    #[test]
    fn analysis_retains_cutoffs_usage_and_uncertain_cost_accounting() {
        let (cases, references, catalog, config, mut responses) = fixture();
        responses[0].execution = Some(crate::domain::ExecutionMetadata {
            input_tokens: Some(80),
            output_tokens: Some(4096),
            truncated: true,
            stop_reason: Some("max_tokens".into()),
            accounted_nanoeur: Some(200_000_000),
            reservation_retained: false,
        });
        responses[1].error = Some("interrupted request".into());
        responses[1].output.clear();
        responses[1].execution = Some(crate::domain::ExecutionMetadata {
            accounted_nanoeur: Some(250_000_000),
            reservation_retained: true,
            ..Default::default()
        });
        let first = input(&config, &cases, &references, &catalog, responses);
        let report = analyze(&cases, &references, &catalog, &[first], 100).unwrap();
        let model = report.models.values().next().unwrap();
        assert_eq!(model.report.overall.classifications["truncated"], 1);
        assert!((model.exact_text.estimate - 0.75).abs() < f64::EPSILON);
        let usage = model.execution.as_ref().unwrap();
        assert_eq!(usage.known_usage_responses, 1);
        assert_eq!(usage.accounted_responses, 2);
        assert_eq!(usage.input_tokens, 80);
        assert_eq!(usage.output_tokens, 4096);
        assert_eq!(usage.accounted_nanoeur, 450_000_000);
        assert_eq!(usage.uncertain_billing_responses, 1);
        let html = crate::visualization::render_html(&[report]).unwrap();
        assert!(html.contains("Usage and cost accounting"));
        assert!(html.contains("Token-limit cutoff"));
    }

    #[test]
    fn examples_distinguish_exact_alternative_from_overlap_and_escape_output() {
        let (cases, references, catalog, config, mut responses) = fixture();
        responses[0].output = references[1].text.clone();
        responses[2].output = "<script>alert('bad')</script>".into();
        let value = input(&config, &cases, &references, &catalog, responses);
        let report = analyze(&cases, &references, &catalog, &[value], 100).unwrap();
        assert_eq!(report.examples[0].exact_alternative_editions, vec!["b"]);
        assert_eq!(
            report
                .models
                .values()
                .next()
                .unwrap()
                .report
                .exact_alternative_matches["a"]["b"],
            1
        );
        assert!(!study_markdown(&report).contains("<script>"));
        assert_eq!(
            word_diff("one two three", "one four three more"),
            "one [-two][+four] three [+more]"
        );
        assert_eq!(word_diff("one two", ""), "[-one] [-two]");
        assert_eq!(word_diff("", "one"), "[+one]");
        assert_eq!(word_diff("one two", "one"), "one [-two]");
    }

    #[test]
    fn example_drilldowns_share_normalized_matches_and_matrix_destinations() {
        let (cases, mut references, catalog, config, mut responses) = fixture();
        references[1].text = "The b\nverse 1.".into();
        responses[0].output = "The b\r\nverse 1.".into();
        responses[2].output.clear();
        responses[4].error = Some("request failed".into());
        responses[4].output.clear();
        let value = input(&config, &cases, &references, &catalog, responses);
        let report = analyze(&cases, &references, &catalog, &[value], 100).unwrap();
        let alternative = report
            .examples
            .iter()
            .find(|e| e.case_id == cases[0].case_id)
            .unwrap();
        assert_eq!(alternative.exact_alternative_editions, vec!["b"]);
        assert_eq!(alternative.resembles.as_deref(), Some("b"));
        for example in &report.examples {
            let matrix = &report.models[&example.model].report.requested_to_resembles;
            assert!(
                matrix[&example.requested_translation][example.resembles.as_ref().unwrap()] > 0
            );
        }
        assert!(
            report
                .examples
                .iter()
                .any(|e| e.resembles.as_deref() == Some("_empty"))
        );
        assert!(
            report
                .examples
                .iter()
                .any(|e| e.resembles.as_deref() == Some("_provider_error"))
        );
        let mut legacy = serde_json::to_value(alternative).unwrap();
        legacy.as_object_mut().unwrap().remove("resembles");
        assert!(
            serde_json::from_value::<Example>(legacy)
                .unwrap()
                .resembles
                .is_none()
        );
    }

    #[test]
    fn private_and_hidden_failures_never_export_text_examples() {
        let (mut cases, references, mut catalog, config, mut responses) = fixture();
        for response in &mut responses {
            response.output = "wrong".into();
        }
        for spec in &mut catalog.translations {
            spec.license_kind = LicenseKind::LicensedPrivate;
            spec.redistribute_reference_text = false;
        }
        let value = input(&config, &cases, &references, &catalog, responses.clone());
        assert!(
            analyze(&cases, &references, &catalog, &[value], 100)
                .unwrap()
                .examples
                .is_empty()
        );
        for spec in &mut catalog.translations {
            spec.license_kind = LicenseKind::PublicDomain;
            spec.redistribute_reference_text = true;
        }
        for (case, response) in cases.iter_mut().zip(&mut responses) {
            case.case_id = case.case_id.replace("BQ-DEV-", "BQ-HID-");
            response.case_id.clone_from(&case.case_id);
        }
        let value = input(&config, &cases, &references, &catalog, responses);
        assert!(
            analyze(&cases, &references, &catalog, &[value], 100)
                .unwrap()
                .examples
                .is_empty()
        );
    }

    #[test]
    fn refuses_mixed_tracks_unbalanced_editions_and_overlaps() {
        let (cases, references, catalog, config, responses) = fixture();
        let mut mixed = cases.clone();
        mixed[0].prompt_variant = PromptVariant::Concise;
        assert!(make_manifest(&config, &mixed, &references, &catalog, &responses).is_err());
        let mut unbalanced = cases.clone();
        unbalanced.remove(0);
        assert!(analyze(&unbalanced, &references, &catalog, &[], 100).is_err());
        let mut inconsistent = cases.clone();
        inconsistent[0].stratum = crate::CaseStratum::ShortVerse;
        assert!(analyze(&inconsistent, &references, &catalog, &[], 100).is_err());
        let mut overlap = cases;
        overlap[0].reference.verse_end = Some(2);
        assert!(reject_overlaps(&overlap).is_err());
        for case in &mut overlap {
            case.reference.verse_end = Some(case.reference.verse_start + 2);
            case.prompt_variant = PromptVariant::CopyControl;
        }
        assert!(track(&overlap).is_err());
    }

    #[test]
    fn mixed_evidence_is_rejected_and_files_roundtrip() {
        let (cases, references, catalog, config, responses) = fixture();
        let first = input(&config, &cases, &references, &catalog, responses);
        let mut second = first.clone();
        second.0.evidence = "synthetic_fixture".into();
        second.0.run_id = "run-2".into();
        for response in &mut second.1 {
            response.run_id = "run-2".into();
        }
        second.0.responses_sha256 = digest(&second.1);
        assert!(
            analyze(&cases, &references, &catalog, &[first.clone(), second], 100)
                .unwrap_err()
                .to_string()
                .contains("synthetic")
        );
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("responses.jsonl");
        write_jsonl(Some(&path), &first.1).unwrap();
        write_json(&manifest_path(&path), &first.0).unwrap();
        analyze_files(
            &cases,
            &references,
            &catalog,
            &[path],
            &temp.path().join("report"),
            100,
        )
        .unwrap();
        assert!(temp.path().join("report/analysis.md").exists());
        assert!(temp.path().join("report/examples.jsonl").exists());
    }

    #[test]
    fn repeated_report_builds_have_identical_fractional_stability_averages() {
        let (cases, references, catalog, mut config, responses) = fixture();
        let mut inputs = Vec::new();
        for repeat in 0..3 {
            config.run_id = format!("repeat-{repeat}");
            let mut produced = responses.clone();
            for (index, response) in produced.iter_mut().enumerate() {
                response.run_id.clone_from(&config.run_id);
                if repeat < index % 4 {
                    response.output = format!("wrong-{repeat}");
                }
            }
            inputs.push(input(&config, &cases, &references, &catalog, produced));
        }
        let first =
            serde_json::to_string(&analyze(&cases, &references, &catalog, &inputs, 100).unwrap())
                .unwrap();
        for _ in 0..20 {
            assert_eq!(
                first,
                serde_json::to_string(
                    &analyze(&cases, &references, &catalog, &inputs, 100).unwrap()
                )
                .unwrap()
            );
        }
    }
}
