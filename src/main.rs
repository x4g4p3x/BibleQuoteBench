use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use biblequotebench::{
    BenchmarkCase, CorpusLock, ReferenceRecord, ResponseRecord, ScoreRecord, TranslationCatalog,
    aggregate_scores,
    importer::import_usfm,
    io::{ensure_nonempty, read_json, read_jsonl, write_json, write_jsonl, write_text},
    provider::{ProviderConfig, ProviderKind},
    report::{build_report, render_markdown},
    sampling::{CuratedReference, SamplingConfig, sample_dataset},
    score_response,
    security::{guard_staged, guard_tracked},
    validate_dataset,
};
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "bqb", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Import a USFM directory or zip and emit a byte-level provenance lock.
    ImportUsfm {
        #[arg(long, default_value = "data/dev/translations.json")]
        translations: PathBuf,
        #[arg(long)]
        translation: String,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        source_url: Option<String>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        lock_output: PathBuf,
    },
    /// Build deterministic, stratified public and hidden dataset splits.
    Sample {
        #[arg(long, default_value = "data/sampling-v0.2.json")]
        config: PathBuf,
        #[arg(long, default_value = "data/dev/translations.json")]
        translations: PathBuf,
        #[arg(long, default_value = "data/strata/famous.jsonl")]
        curated: PathBuf,
        #[arg(long, required = true)]
        corpus: Vec<PathBuf>,
        #[arg(long, required = true)]
        lock: Vec<PathBuf>,
        #[arg(long, default_value = "data/dev/cases.jsonl")]
        dev_cases: PathBuf,
        #[arg(long, default_value = "data/dev/references.jsonl")]
        dev_references: PathBuf,
        #[arg(long, default_value = "data/hidden/cases.jsonl")]
        hidden_cases: PathBuf,
        #[arg(long, default_value = "data/hidden/references.jsonl")]
        hidden_references: PathBuf,
        #[arg(long, default_value = "data/release/v0.2-manifest.json")]
        manifest: PathBuf,
        #[arg(long, default_value = "data/hidden/sampling-secret.txt")]
        hidden_seed_file: PathBuf,
    },
    /// Validate translation metadata, cases, and reference-text coverage.
    Validate(DatasetPaths),
    /// Render the exact prompt for one benchmark case.
    Prompt {
        #[command(flatten)]
        dataset: DatasetPaths,
        #[arg(long)]
        case_id: String,
    },
    /// Run closed-book prompts using a live model provider.
    Run {
        #[command(flatten)]
        dataset: DatasetPaths,
        #[arg(long, value_enum)]
        provider: ProviderKind,
        #[arg(long)]
        model: String,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        api_key_env: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        temperature: Option<f32>,
        /// Explicit Responses or Claude effort; recorded without substitution.
        #[arg(long)]
        reasoning_effort: Option<String>,
        #[arg(long, default_value_t = 4096)]
        max_output_tokens: u32,
        #[arg(long)]
        case_limit: Option<usize>,
        #[arg(long)]
        fail_fast: bool,
        /// Price assumptions and a shared campaign ceiling; mandatory for live endpoints.
        #[arg(long)]
        budget: Option<PathBuf>,
        /// Explicitly authorize an enabled live policy.
        #[arg(long)]
        allow_paid: bool,
        #[arg(long)]
        resume: bool,
        /// Check inputs and estimate reservations without credentials or network access.
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        output: PathBuf,
    },
    /// Score model responses and emit one JSON object per response.
    Score {
        #[command(flatten)]
        dataset: DatasetPaths,
        #[arg(long)]
        responses: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Aggregate an existing score JSONL file as JSON.
    Summarize {
        #[arg(long)]
        scores: PathBuf,
    },
    /// Generate JSON and Markdown leaderboard analysis.
    Report {
        #[arg(long)]
        scores: PathBuf,
        #[arg(long, default_value = "results/report.md")]
        markdown: PathBuf,
        #[arg(long, default_value = "results/report.json")]
        json: PathBuf,
    },
    /// Analyze complete, manifest-bound runs with paired cluster confidence intervals.
    Analyze {
        #[command(flatten)]
        dataset: DatasetPaths,
        #[arg(long, required = true)]
        responses: Vec<PathBuf>,
        #[arg(long, default_value = "results/analysis")]
        output_dir: PathBuf,
        #[arg(long, default_value_t = 2000)]
        resamples: usize,
    },
    /// Render validated analysis JSON files as one portable, offline interactive report.
    Visualize {
        #[arg(long, required = true)]
        analysis: Vec<PathBuf>,
        #[arg(long, default_value = "results/report.html")]
        output: PathBuf,
    },
    /// Prepare public prompt controls and three-verse passage diagnostics.
    PreparePilot {
        #[command(flatten)]
        dataset: DatasetPaths,
        #[arg(long, required = true)]
        corpus: Vec<PathBuf>,
        #[arg(long, default_value_t = 12)]
        reference_count: usize,
        #[arg(long, default_value = "data/pilot/v0.2")]
        output_dir: PathBuf,
    },
    /// Produce an explicitly synthetic pilot without any network or paid calls.
    SyntheticPilot {
        #[arg(long, default_value = "data/pilot/v0.2")]
        dataset_dir: PathBuf,
        #[arg(long, default_value = "docs/pilot/v0.2")]
        output_dir: PathBuf,
    },
    /// Reject private benchmark material or credentials in staged changes.
    GuardStaged,
    /// Reject private benchmark material or credentials anywhere in the Git index.
    GuardTracked,
}

#[derive(Debug, Clone, Args)]
struct DatasetPaths {
    #[arg(long, default_value = "data/dev/translations.json")]
    translations: PathBuf,
    #[arg(long, default_value = "data/dev/cases.jsonl")]
    cases: PathBuf,
    #[arg(long, default_value = "data/dev/references.jsonl")]
    references: PathBuf,
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    match Cli::parse().command {
        Command::ImportUsfm {
            translations,
            translation,
            source,
            source_url,
            output,
            lock_output,
        } => import_command(
            &translations,
            &translation,
            &source,
            source_url.as_deref(),
            &output,
            &lock_output,
        ),
        Command::Sample {
            config,
            translations,
            curated,
            corpus,
            lock,
            dev_cases,
            dev_references,
            hidden_cases,
            hidden_references,
            manifest,
            hidden_seed_file,
        } => sample_command(
            &config,
            &translations,
            &curated,
            &corpus,
            &lock,
            &dev_cases,
            &dev_references,
            &hidden_cases,
            &hidden_references,
            &manifest,
            &hidden_seed_file,
        ),
        Command::Validate(paths) => validate_command(&paths),
        Command::Prompt {
            dataset: paths,
            case_id,
        } => prompt_command(&paths, &case_id),
        Command::Run {
            dataset: paths,
            provider,
            model,
            run_id,
            api_key_env,
            base_url,
            temperature,
            reasoning_effort,
            max_output_tokens,
            case_limit,
            fail_fast,
            budget,
            allow_paid,
            resume,
            dry_run,
            output,
        } => run_command(
            &paths,
            &ProviderConfig {
                kind: provider,
                model,
                run_id,
                api_key_env,
                base_url,
                temperature,
                reasoning_effort,
                max_output_tokens,
                case_limit,
                fail_fast,
            },
            &output,
            &biblequotebench::execution::RunOptions {
                budget,
                allow_paid,
                resume,
                dry_run,
            },
        ),
        Command::Score {
            dataset: paths,
            responses,
            output,
        } => score_command(&paths, &responses, output.as_deref()),
        Command::Summarize { scores } => summarize_command(&scores),
        Command::Report {
            scores,
            markdown,
            json,
        } => report_command(&scores, &markdown, &json),
        Command::Analyze {
            dataset,
            responses,
            output_dir,
            resamples,
        } => {
            let dataset = load_dataset(&dataset)?;
            biblequotebench::study::analyze_files(
                &dataset.cases,
                &dataset.references,
                &dataset.catalog,
                &responses,
                &output_dir,
                resamples,
            )
        }
        Command::Visualize { analysis, output } => {
            let reports = analysis
                .iter()
                .map(|path| read_json(path))
                .collect::<Result<Vec<_>>>()?;
            biblequotebench::visualization::write_html(&output, &reports)
        }
        Command::PreparePilot {
            dataset,
            corpus,
            reference_count,
            output_dir,
        } => {
            let dataset = load_dataset(&dataset)?;
            let corpus = read_many_jsonl(&corpus)?;
            biblequotebench::pilot::prepare(
                &dataset.cases,
                &corpus,
                &dataset.catalog,
                reference_count,
                &output_dir,
            )
        }
        Command::SyntheticPilot {
            dataset_dir,
            output_dir,
        } => biblequotebench::pilot::synthetic(&dataset_dir, &output_dir),
        Command::GuardStaged => guard_command(false),
        Command::GuardTracked => guard_command(true),
    }
}

fn guard_command(all_tracked: bool) -> Result<()> {
    let violations = if all_tracked {
        guard_tracked()?
    } else {
        guard_staged()?
    };
    if violations.is_empty() {
        println!("publication guard passed");
        return Ok(());
    }
    let details = violations
        .iter()
        .map(|violation| format!("- {}: {}", violation.path, violation.reason))
        .collect::<Vec<_>>()
        .join("\n");
    bail!("publication guard blocked the change:\n{details}")
}

fn import_command(
    translations_path: &Path,
    translation_id: &str,
    source: &Path,
    source_url: Option<&str>,
    output: &Path,
    lock_output: &Path,
) -> Result<()> {
    let catalog: TranslationCatalog = read_json(translations_path)?;
    let translation = catalog
        .translations
        .iter()
        .find(|translation| translation.id == translation_id)
        .with_context(|| format!("unknown translation: {translation_id}"))?;
    let imported = import_usfm(
        source,
        translation,
        source_url.unwrap_or(&translation.source_url),
    )?;
    ensure_parent(output)?;
    ensure_parent(lock_output)?;
    write_jsonl(Some(output), &imported.records)?;
    write_json(lock_output, &imported.lock)?;
    println!(
        "imported {} references for {} (corpus sha256 {})",
        imported.records.len(),
        translation_id,
        imported.lock.corpus_sha256
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn sample_command(
    config_path: &Path,
    translations_path: &Path,
    curated_path: &Path,
    corpus_paths: &[PathBuf],
    lock_paths: &[PathBuf],
    dev_cases_path: &Path,
    dev_references_path: &Path,
    hidden_cases_path: &Path,
    hidden_references_path: &Path,
    manifest_path: &Path,
    hidden_seed_path: &Path,
) -> Result<()> {
    let config: SamplingConfig = read_json(config_path)?;
    let catalog: TranslationCatalog = read_json(translations_path)?;
    let curated: Vec<CuratedReference> = read_jsonl(curated_path)?;
    let hidden_seed = fs::read_to_string(hidden_seed_path)
        .with_context(|| format!("reading private hidden seed {}", hidden_seed_path.display()))?;
    let references = read_many_jsonl(corpus_paths)?;
    let locks: Vec<CorpusLock> = lock_paths
        .iter()
        .map(|path| read_json(path))
        .collect::<Result<_>>()?;
    let translation_ids: Vec<String> = catalog
        .translations
        .iter()
        .map(|item| item.id.clone())
        .collect();
    let sampled = sample_dataset(
        &config,
        &translation_ids,
        &references,
        &locks,
        &curated,
        hidden_seed.trim(),
    )?;
    for path in [
        dev_cases_path,
        dev_references_path,
        hidden_cases_path,
        hidden_references_path,
        manifest_path,
    ] {
        ensure_parent(path)?;
    }
    write_jsonl(Some(dev_cases_path), &sampled.dev_cases)?;
    write_jsonl(Some(dev_references_path), &sampled.dev_references)?;
    write_jsonl(Some(hidden_cases_path), &sampled.hidden_cases)?;
    write_jsonl(Some(hidden_references_path), &sampled.hidden_references)?;
    write_json(manifest_path, &sampled.manifest)?;
    println!(
        "sampled {} public and {} hidden cases; hidden commitment {}",
        sampled.dev_cases.len(),
        sampled.hidden_cases.len(),
        sampled.manifest.hidden_cases_sha256
    );
    Ok(())
}

fn validate_command(paths: &DatasetPaths) -> Result<()> {
    let dataset = load_dataset(paths)?;
    validate_dataset(&dataset.catalog, &dataset.cases, &dataset.references)?;
    println!(
        "valid: {} translations, {} cases, {} reference records",
        dataset.catalog.translations.len(),
        dataset.cases.len(),
        dataset.references.len()
    );
    Ok(())
}

fn prompt_command(paths: &DatasetPaths, case_id: &str) -> Result<()> {
    let dataset = load_dataset(paths)?;
    validate_dataset(&dataset.catalog, &dataset.cases, &dataset.references)?;
    let case = dataset
        .cases
        .iter()
        .find(|case| case.case_id == case_id)
        .with_context(|| format!("unknown case_id: {case_id}"))?;
    let translation = dataset
        .catalog
        .translations
        .iter()
        .find(|translation| translation.id == case.translation)
        .expect("validated dataset guarantees translation coverage");
    let supplied = dataset
        .references
        .iter()
        .find(|record| record.translation == case.translation && record.reference == case.reference)
        .map(|record| record.text.as_str());
    println!(
        "{}",
        biblequotebench::prompt::execution_prompt(case, translation, supplied)?
    );
    Ok(())
}

fn run_command(
    paths: &DatasetPaths,
    config: &ProviderConfig,
    output: &Path,
    options: &biblequotebench::execution::RunOptions,
) -> Result<()> {
    let dataset = load_dataset(paths)?;
    biblequotebench::execution::execute(
        config,
        &dataset.cases,
        &dataset.references,
        &dataset.catalog,
        output,
        options,
    )
}

fn score_command(paths: &DatasetPaths, responses: &Path, output: Option<&Path>) -> Result<()> {
    let dataset = load_dataset(paths)?;
    validate_dataset(&dataset.catalog, &dataset.cases, &dataset.references)?;
    let response_records: Vec<ResponseRecord> = read_jsonl(responses)?;
    ensure_nonempty(&response_records, "responses", responses)?;
    let scores = score_all(&dataset.cases, &dataset.references, &response_records)?;
    if let Some(path) = output {
        ensure_parent(path)?;
    }
    write_jsonl(output, &scores)
}

fn summarize_command(scores: &Path) -> Result<()> {
    let records: Vec<ScoreRecord> = read_jsonl(scores)?;
    ensure_nonempty(&records, "scores", scores)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&aggregate_scores(&records))?
    );
    Ok(())
}

fn report_command(scores: &Path, markdown: &Path, json: &Path) -> Result<()> {
    let records: Vec<ScoreRecord> = read_jsonl(scores)?;
    ensure_nonempty(&records, "scores", scores)?;
    let report = build_report(&records);
    ensure_parent(markdown)?;
    ensure_parent(json)?;
    write_text(markdown, &render_markdown(&report))?;
    write_json(json, &report)?;
    println!("wrote {} and {}", markdown.display(), json.display());
    Ok(())
}

struct Dataset {
    catalog: TranslationCatalog,
    cases: Vec<BenchmarkCase>,
    references: Vec<ReferenceRecord>,
}

fn load_dataset(paths: &DatasetPaths) -> Result<Dataset> {
    Ok(Dataset {
        catalog: read_json(&paths.translations)?,
        cases: read_jsonl(&paths.cases)?,
        references: read_jsonl(&paths.references)?,
    })
}

fn read_many_jsonl<T: serde::de::DeserializeOwned>(paths: &[PathBuf]) -> Result<Vec<T>> {
    let mut records = Vec::new();
    for path in paths {
        records.extend(read_jsonl(path)?);
    }
    Ok(records)
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    Ok(())
}

fn score_all(
    cases: &[BenchmarkCase],
    references: &[ReferenceRecord],
    responses: &[ResponseRecord],
) -> Result<Vec<ScoreRecord>> {
    let cases_by_id: HashMap<&str, &BenchmarkCase> = cases
        .iter()
        .map(|case| (case.case_id.as_str(), case))
        .collect();
    let mut response_keys = HashSet::new();
    let mut scores = Vec::with_capacity(responses.len());
    for response in responses {
        let response_key = (
            response.case_id.as_str(),
            response.run_id.as_str(),
            response.provider.as_str(),
            response.model.as_str(),
        );
        if !response_keys.insert(response_key) {
            bail!(
                "duplicate response for case_id {}, run_id {}, provider {}, and model {}",
                response.case_id,
                response.run_id,
                response.provider,
                response.model
            );
        }
        let case = cases_by_id
            .get(response.case_id.as_str())
            .with_context(|| format!("response uses unknown case_id: {}", response.case_id))?;
        let requested = references
            .iter()
            .find(|record| {
                record.translation == case.translation && record.reference == case.reference
            })
            .expect("validated dataset guarantees requested reference text");
        let alternatives: Vec<&ReferenceRecord> = references
            .iter()
            .filter(|record| {
                record.reference == case.reference && record.translation != case.translation
            })
            .collect();
        scores.push(score_response(case, response, requested, &alternatives));
    }
    Ok(scores)
}

#[cfg(test)]
mod tests {
    use biblequotebench::{BibleReference, CaseStratum, PromptVariant};

    use super::*;

    #[test]
    fn rejects_duplicate_run_for_case_and_model() {
        let reference = BibleReference {
            book: "Test".to_owned(),
            chapter: 1,
            verse_start: 1,
            verse_end: None,
        };
        let cases = vec![BenchmarkCase {
            case_id: "BQ-1".to_owned(),
            translation: "t1".to_owned(),
            reference: reference.clone(),
            stratum: CaseStratum::Random,
            prompt_variant: PromptVariant::Canonical,
        }];
        let references = vec![ReferenceRecord {
            translation: "t1".to_owned(),
            reference,
            text: "Text".to_owned(),
        }];
        let response = ResponseRecord {
            case_id: "BQ-1".to_owned(),
            run_id: "run-1".to_owned(),
            provider: "fixture".to_owned(),
            model: "fixture".to_owned(),
            resolved_model: None,
            output: "Text".to_owned(),
            error: None,
            temperature: Some(0.0),
            reasoning_effort: None,
            seed: None,
            provider_request_id: None,
            system_fingerprint: None,
            execution: None,
        };
        let error = score_all(&cases, &references, &[response.clone(), response]).unwrap_err();
        assert!(error.to_string().contains("duplicate response"));
    }
}
