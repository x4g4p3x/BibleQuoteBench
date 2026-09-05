//! Durable, budget-bound execution. An uncertain request is never replayed automatically.

use crate::{
    BenchmarkCase, ReferenceRecord, ResponseRecord, TranslationCatalog,
    io::{read_json, write_json, write_jsonl},
    prompt::execution_prompt,
    provider::{ProviderConfig, run_cases_with_supplied, validate_config},
    study::{RunManifest, digest, make_manifest, manifest_path},
};
use anyhow::{Context, Result, bail};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

/// Explicit local price assumptions. Integer nano-euros avoid rounding overspend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetPolicy {
    pub provider: String,
    pub model: String,
    pub execution_enabled: bool,
    pub limit_nanoeur: u64,
    pub input_nanoeur_per_token: u64,
    pub output_nanoeur_per_token: u64,
    pub pricing_source: String,
    pub pricing_checked: String,
}

impl BudgetPolicy {
    fn validate(&self, config: &ProviderConfig) -> Result<()> {
        if self.provider != config.kind.name() || self.model != config.model {
            bail!("budget policy does not match provider and model");
        }
        if self.limit_nanoeur == 0
            || self.input_nanoeur_per_token == 0
            || self.output_nanoeur_per_token == 0
            || self.pricing_source.is_empty()
            || self.pricing_checked.is_empty()
        {
            bail!("budget requires positive ceiling, prices, and pricing provenance");
        }
        Ok(())
    }

    fn cost(&self, input: u64, output: u64) -> Result<u64> {
        input
            .checked_mul(self.input_nanoeur_per_token)
            .and_then(|n| {
                output
                    .checked_mul(self.output_nanoeur_per_token)
                    .and_then(|o| n.checked_add(o))
            })
            .context("budget arithmetic overflow")
    }
}

#[derive(Debug, Default)]
pub struct RunOptions {
    pub budget: Option<PathBuf>,
    pub allow_paid: bool,
    pub resume: bool,
    pub dry_run: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    case_id: String,
    reserved_nanoeur: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedRun {
    manifest: RunManifest,
    output: PathBuf,
    limit: usize,
    records: Vec<ResponseRecord>,
    charges_nanoeur: Vec<u64>,
    pending: Option<Pending>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    schema_version: u16,
    policy_sha256: String,
    charged_nanoeur: u64,
    runs: BTreeMap<String, SavedRun>,
}

fn euros(nanoeur: u64) -> String {
    format!(
        "EUR {}.{:06}",
        nanoeur / 1_000_000_000,
        (nanoeur % 1_000_000_000) / 1000
    )
}

fn atomic_save(path: &Path, ledger: &Ledger) -> Result<()> {
    let temporary = path.with_extension("checkpoint.tmp");
    let mut file = File::create(&temporary)?;
    serde_json::to_writer(&mut file, ledger)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, path).context("replacing durable execution checkpoint")
}

fn lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    file.try_lock_exclusive()
        .context("another process is using this budget or output")?;
    Ok(file)
}

fn is_loopback(config: &ProviderConfig) -> bool {
    reqwest::Url::parse(
        config
            .base_url
            .as_deref()
            .unwrap_or(config.kind.default_base_url()),
    )
    .is_ok_and(|url| {
        url.scheme() == "http"
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
    })
}

fn reservation(
    policy: &BudgetPolicy,
    config: &ProviderConfig,
    case: &BenchmarkCase,
    catalog: &TranslationCatalog,
    references: &[ReferenceRecord],
) -> Result<u64> {
    let translation = catalog
        .translations
        .iter()
        .find(|t| t.id == case.translation)
        .context("missing edition")?;
    let supplied = references
        .iter()
        .find(|r| r.translation == case.translation && r.reference == case.reference)
        .map(|r| r.text.as_str());
    let prompt = execution_prompt(case, translation, supplied)?;
    // Text-only requests: byte count is deliberately more conservative than token estimates.
    // Leave additional room for the API's message framing. No caching or paid tools are requested.
    let input_bound = u64::try_from(prompt.len())?
        .checked_add(4096)
        .context("input bound overflow")?;
    policy.cost(input_bound, u64::from(config.max_output_tokens))
}

fn error_record(config: &ProviderConfig, case_id: &str, message: String) -> ResponseRecord {
    ResponseRecord {
        case_id: case_id.into(),
        run_id: config.run_id.clone(),
        provider: config.kind.name().into(),
        model: config.model.clone(),
        resolved_model: None,
        output: String::new(),
        error: Some(message),
        temperature: config.temperature,
        reasoning_effort: config.reasoning_effort.clone(),
        seed: None,
        provider_request_id: None,
        system_fingerprint: None,
        execution: None,
    }
}

/// Runs a campaign with shared budget accounting, durable checkpoints, and resumable outputs.
/// Dry runs never access credentials, write files, or make network requests.
///
/// # Errors
/// Rejects unapproved execution, changed resume inputs, concurrent writers, and insufficient funds.
///
/// # Panics
/// Panics if an internal invariant removes the active run from the locked ledger.
#[allow(clippy::too_many_lines)]
pub fn execute(
    config: &ProviderConfig,
    cases: &[BenchmarkCase],
    references: &[ReferenceRecord],
    catalog: &TranslationCatalog,
    output: &Path,
    options: &RunOptions,
) -> Result<()> {
    validate_config(config)?;
    if output.extension().is_none_or(|ext| ext != "jsonl") {
        bail!("response output must have a .jsonl extension");
    }
    if !is_loopback(config)
        && !config
            .base_url
            .as_deref()
            .unwrap_or(config.kind.default_base_url())
            .starts_with("https://")
    {
        bail!("non-loopback endpoints require HTTPS");
    }
    let expected = make_manifest(config, cases, references, catalog, &[])?;
    let limit = config.case_limit.unwrap_or(cases.len()).min(cases.len());
    if limit == 0 {
        bail!("run must contain at least one case");
    }
    let local = is_loopback(config);
    let policy = if let Some(path) = &options.budget {
        read_json::<BudgetPolicy>(path)?
    } else {
        if !local {
            bail!("live execution requires --budget with verified price assumptions");
        }
        BudgetPolicy {
            provider: config.kind.name().into(),
            model: config.model.clone(),
            execution_enabled: true,
            limit_nanoeur: 1_000_000_000,
            input_nanoeur_per_token: 1,
            output_nanoeur_per_token: 1,
            pricing_source: "loopback test accounting; no monetary charge".into(),
            pricing_checked: "local".into(),
        }
    };
    policy.validate(config)?;
    let reservations: Vec<_> = cases
        .iter()
        .take(limit)
        .map(|case| reservation(&policy, config, case, catalog, references))
        .collect::<Result<_>>()?;
    if options.dry_run {
        let upper = reservations.iter().try_fold(0_u64, |total, value| {
            total
                .checked_add(*value)
                .context("budget arithmetic overflow")
        })?;
        println!(
            "Prepared {limit} requests; conservative reservation total {}; campaign ceiling {}. No requests sent. Execution enabled: {}",
            euros(upper),
            euros(policy.limit_nanoeur),
            policy.execution_enabled
        );
        return Ok(());
    }
    if !policy.execution_enabled || (!local && !options.allow_paid) {
        bail!(
            "execution is held: enable the approved policy and pass --allow-paid for a live endpoint"
        );
    }
    if let Some(name) = config
        .api_key_env
        .as_deref()
        .or_else(|| config.kind.default_api_key_env())
    {
        if std::env::var(name).map_or(true, |key| key.trim().is_empty()) {
            bail!("credential environment variable {name} is not configured");
        }
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let output = parent
        .canonicalize()?
        .join(output.file_name().context("output needs a filename")?);
    let ledger_path = options.budget.as_ref().map_or_else(
        || output.with_extension("ledger.json"),
        |p| p.with_extension("ledger.json"),
    );
    if let Some(policy_path) = &options.budget {
        if policy_path.canonicalize()? == manifest_path(&output) {
            bail!("manifest path must not overwrite the budget policy");
        }
    }
    let _budget_lock = lock(&ledger_path.with_extension("ledger.lock"))?;
    let _output_lock = lock(&output.with_extension("output.lock"))?;
    let mut ledger = if ledger_path.exists() {
        read_json::<Ledger>(&ledger_path)?
    } else {
        Ledger {
            schema_version: 1,
            policy_sha256: digest(&policy),
            charged_nanoeur: 0,
            runs: BTreeMap::new(),
        }
    };
    if ledger.schema_version != 1 || ledger.policy_sha256 != digest(&policy) {
        bail!("budget policy differs from the saved campaign; restore its original settings");
    }
    let accounted = ledger.runs.values().try_fold(0_u64, |total, run| {
        if run.records.len() != run.charges_nanoeur.len() {
            bail!("invalid checkpoint charges");
        }
        run.charges_nanoeur
            .iter()
            .copied()
            .chain(run.pending.as_ref().map(|p| p.reserved_nanoeur))
            .try_fold(total, |n, charge| {
                n.checked_add(charge).context("checkpoint charge overflow")
            })
    })?;
    if accounted != ledger.charged_nanoeur {
        bail!("checkpoint budget accounting mismatch");
    }
    let key = format!("{} / {}", config.kind.name(), config.run_id);
    if let Some(saved) = ledger.runs.get(&key) {
        if !options.resume {
            bail!("run already exists; use --resume");
        }
        if saved.manifest != expected || saved.limit != limit || saved.output != output {
            bail!("resume dataset, settings, case limit, or output differs from checkpoint");
        }
        if saved.records.len() > limit
            || saved.records.iter().zip(cases).any(|(record, case)| {
                record.case_id != case.case_id
                    || record.run_id != config.run_id
                    || record.model != config.model
            })
        {
            bail!("invalid checkpoint response sequence");
        }
    } else {
        if options.resume {
            bail!("no saved run exists to resume");
        }
        if output.exists() || manifest_path(&output).exists() {
            bail!("output or manifest already exists");
        }
        ledger.runs.insert(
            key.clone(),
            SavedRun {
                manifest: expected,
                output: output.clone(),
                limit,
                records: Vec::new(),
                charges_nanoeur: Vec::new(),
                pending: None,
            },
        );
        atomic_save(&ledger_path, &ledger)?;
    }
    // An interrupted request may have been billed. Keep its reservation and record uncertainty.
    if let Some(pending) = ledger.runs.get_mut(&key).expect("saved run").pending.take() {
        let saved = ledger.runs.get_mut(&key).expect("saved run");
        if cases
            .get(saved.records.len())
            .is_none_or(|case| case.case_id != pending.case_id)
        {
            bail!("invalid pending checkpoint");
        }
        saved.charges_nanoeur.push(pending.reserved_nanoeur);
        saved.records.push(error_record(config, &pending.case_id, "interrupted request: outcome and billing unknown; not replayed; full reservation retained".into()));
        let meta = saved
            .records
            .last_mut()
            .expect("interrupted record")
            .execution
            .get_or_insert_with(Default::default);
        meta.accounted_nanoeur = Some(pending.reserved_nanoeur);
        meta.reservation_retained = true;
        atomic_save(&ledger_path, &ledger)?;
    }
    write_jsonl(Some(&output), &ledger.runs[&key].records)?;
    let mut single = config.clone();
    single.case_limit = None;
    single.fail_fast = false;
    for index in ledger.runs[&key].records.len()..limit {
        let reserved = reservations[index];
        let total = ledger
            .charged_nanoeur
            .checked_add(reserved)
            .context("budget arithmetic overflow")?;
        if total > policy.limit_nanoeur {
            bail!(
                "budget stopped before case {}: accounted {} nano-EUR, next reservation {reserved}, ceiling {}; progress saved; no request sent",
                cases[index].case_id,
                ledger.charged_nanoeur,
                policy.limit_nanoeur
            );
        }
        ledger.charged_nanoeur = total;
        ledger.runs.get_mut(&key).expect("saved run").pending = Some(Pending {
            case_id: cases[index].case_id.clone(),
            reserved_nanoeur: reserved,
        });
        atomic_save(&ledger_path, &ledger)?;
        let mut record =
            match run_cases_with_supplied(&single, &cases[index..=index], catalog, references) {
                Ok(mut records) => records.remove(0),
                Err(error) => error_record(config, &cases[index].case_id, format!("{error:#}")),
            };
        let observed = record
            .execution
            .as_ref()
            .and_then(|meta| meta.input_tokens.zip(meta.output_tokens));
        let before = ledger.charged_nanoeur - reserved;
        let (charged, invalid_usage) =
            match observed.map_or(Ok(reserved), |(input, output)| policy.cost(input, output)) {
                Ok(cost) if before.checked_add(cost).is_some() => (cost, false),
                _ => (reserved, true),
            };
        ledger.charged_nanoeur = before + charged;
        let meta = record.execution.get_or_insert_with(Default::default);
        meta.accounted_nanoeur = Some(charged);
        meta.reservation_retained = observed.is_none() || invalid_usage;
        let failed = record.error.is_some();
        let saved = ledger.runs.get_mut(&key).expect("saved run");
        saved.records.push(record);
        saved.charges_nanoeur.push(charged);
        saved.pending = None;
        atomic_save(&ledger_path, &ledger)?;
        write_jsonl(Some(&output), &ledger.runs[&key].records)?;
        println!(
            "Saved {}/{} responses; accounted {} / {} (unknown usage retains its reservation)",
            index + 1,
            limit,
            euros(ledger.charged_nanoeur),
            euros(policy.limit_nanoeur)
        );
        if invalid_usage {
            bail!(
                "provider usage could not be priced; response saved, reservation retained, execution stopped"
            );
        }
        if charged > reserved {
            bail!(
                "provider usage exceeded the conservative reservation; saved response and stopped; review pricing and token bounds"
            );
        }
        if failed && config.fail_fast {
            bail!(
                "provider failed; response and budget saved; --resume continues with the next case"
            );
        }
    }
    let manifest = make_manifest(
        config,
        cases,
        references,
        catalog,
        &ledger.runs[&key].records,
    )?;
    write_json(&manifest_path(&output), &manifest)?;
    println!(
        "Run complete; response manifest written. Campaign accounted {}.",
        euros(ledger.charged_nanoeur)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{io::read_jsonl, provider::ProviderKind};
    use std::{io::Read, net::TcpListener, thread};

    fn dataset() -> (TranslationCatalog, Vec<BenchmarkCase>, Vec<ReferenceRecord>) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/pilot/v0.2/canonical");
        (
            read_json(&root.join("translations.json")).unwrap(),
            read_jsonl(&root.join("cases.jsonl")).unwrap(),
            read_jsonl(&root.join("references.jsonl")).unwrap(),
        )
    }

    fn config(url: &str) -> ProviderConfig {
        ProviderConfig {
            kind: ProviderKind::OpenaiCompatible,
            model: "fixture".into(),
            run_id: "run-1".into(),
            api_key_env: None,
            base_url: Some(url.into()),
            temperature: Some(0.0),
            reasoning_effort: None,
            max_output_tokens: 100,
            case_limit: Some(2),
            fail_fast: false,
        }
    }

    fn policy(config: &ProviderConfig, limit: u64) -> BudgetPolicy {
        BudgetPolicy {
            provider: config.kind.name().into(),
            model: config.model.clone(),
            execution_enabled: true,
            limit_nanoeur: limit,
            input_nanoeur_per_token: 1,
            output_nanoeur_per_token: 10,
            pricing_source: "fixture".into(),
            pricing_checked: "fixture".into(),
        }
    }

    fn server(bodies: Vec<&'static str>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            for body in bodies {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                    .unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut buffer = [0_u8; 4096];
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let length: usize = String::from_utf8_lossy(&bytes[..end])
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|n| n.trim().parse().unwrap())
                            })
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        (url, worker)
    }

    const ANSWER: &str = r#"{"model":"fixture","choices":[{"message":{"content":"answer"},"finish_reason":"stop"}],"usage":{"prompt_tokens":20,"completion_tokens":10}}"#;

    #[test]
    fn observed_usage_releases_reservation_and_completed_resume_sends_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let (catalog, cases, refs) = dataset();
        let (url, worker) = server(vec![ANSWER, ANSWER]);
        let config = config(&url);
        let budget = temp.path().join("budget.json");
        write_json(&budget, &policy(&config, 6000)).unwrap();
        let mut options = RunOptions {
            budget: Some(budget.clone()),
            ..RunOptions::default()
        };
        let output = temp.path().join("run.jsonl");
        execute(&config, &cases, &refs, &catalog, &output, &options).unwrap();
        worker.join().unwrap();
        let ledger: Ledger = read_json(&budget.with_extension("ledger.json")).unwrap();
        assert_eq!(ledger.charged_nanoeur, 240);
        assert_eq!(read_jsonl::<ResponseRecord>(&output).unwrap().len(), 2);
        // Rebuild a damaged export from the durable journal without contacting the closed server.
        fs::write(&output, "partial").unwrap();
        options.resume = true;
        execute(&config, &cases, &refs, &catalog, &output, &options).unwrap();
        assert_eq!(read_jsonl::<ResponseRecord>(&output).unwrap().len(), 2);
        let mut changed = config.clone();
        changed.max_output_tokens += 1;
        assert!(
            execute(&changed, &cases, &refs, &catalog, &output, &options)
                .unwrap_err()
                .to_string()
                .contains("differs")
        );
        let mut next = config.clone();
        next.run_id = "run-2".into();
        let tiny = policy(&next, 10);
        assert!(tiny.cost(u64::MAX, 1).is_err());
    }

    #[test]
    fn uncertain_usage_retains_reservation_and_budget_stops_before_next_request() {
        let temp = tempfile::tempdir().unwrap();
        let (catalog, cases, refs) = dataset();
        let (url, worker) = server(vec![r#"{"choices":[{"message":{"content":"answer"}}]}"#]);
        let config = config(&url);
        let mut budget_policy = policy(&config, 1);
        let reserved = reservation(&budget_policy, &config, &cases[0], &catalog, &refs).unwrap();
        budget_policy.limit_nanoeur = reserved;
        let budget = temp.path().join("budget.json");
        write_json(&budget, &budget_policy).unwrap();
        let options = RunOptions {
            budget: Some(budget.clone()),
            ..RunOptions::default()
        };
        let output = temp.path().join("run.jsonl");
        assert!(
            execute(&config, &cases, &refs, &catalog, &output, &options)
                .unwrap_err()
                .to_string()
                .contains("budget stopped")
        );
        worker.join().unwrap();
        assert_eq!(read_jsonl::<ResponseRecord>(&output).unwrap().len(), 1);
        assert!(!manifest_path(&output).exists());
        let ledger: Ledger = read_json(&budget.with_extension("ledger.json")).unwrap();
        assert_eq!(ledger.charged_nanoeur, reserved);
        let mut other = config.clone();
        other.run_id = "another-run".into();
        assert!(
            execute(
                &other,
                &cases,
                &refs,
                &catalog,
                &temp.path().join("other.jsonl"),
                &options
            )
            .unwrap_err()
            .to_string()
            .contains("budget stopped")
        );
    }

    #[test]
    fn held_policy_dry_run_and_exclusive_locks_are_enforced_without_network() {
        let temp = tempfile::tempdir().unwrap();
        let (catalog, cases, refs) = dataset();
        let config = config("https://example.invalid/v1");
        let mut held = policy(&config, 10000);
        held.execution_enabled = false;
        let budget = temp.path().join("budget.json");
        write_json(&budget, &held).unwrap();
        let mut options = RunOptions {
            budget: Some(budget.clone()),
            dry_run: true,
            ..RunOptions::default()
        };
        let output = temp.path().join("run.jsonl");
        execute(&config, &cases, &refs, &catalog, &output, &options).unwrap();
        assert!(!output.exists());
        assert!(!budget.with_extension("ledger.json").exists());
        options.dry_run = false;
        options.allow_paid = true;
        assert!(
            execute(&config, &cases, &refs, &catalog, &output, &options)
                .unwrap_err()
                .to_string()
                .contains("held")
        );
        held.execution_enabled = true;
        write_json(&budget, &held).unwrap();
        options.allow_paid = false;
        assert!(
            execute(&config, &cases, &refs, &catalog, &output, &options)
                .unwrap_err()
                .to_string()
                .contains("held")
        );
        options.budget = None;
        assert!(
            execute(&config, &cases, &refs, &catalog, &output, &options)
                .unwrap_err()
                .to_string()
                .contains("--budget")
        );
        let path = temp.path().join("exclusive.lock");
        let guard = lock(&path).unwrap();
        assert!(lock(&path).is_err());
        drop(guard);
        assert!(lock(&path).is_ok());
    }

    #[test]
    fn invalid_billing_counters_stop_after_saving_the_received_response() {
        let temp = tempfile::tempdir().unwrap();
        let (catalog, cases, refs) = dataset();
        let (url, worker) = server(vec![
            r#"{"choices":[{"message":{"content":"retained answer"}}],"usage":{"prompt_tokens":18446744073709551615,"completion_tokens":10}}"#,
        ]);
        let config = config(&url);
        let budget = temp.path().join("budget.json");
        write_json(&budget, &policy(&config, 10000)).unwrap();
        let options = RunOptions {
            budget: Some(budget),
            ..RunOptions::default()
        };
        let output = temp.path().join("run.jsonl");
        assert!(
            execute(&config, &cases, &refs, &catalog, &output, &options)
                .unwrap_err()
                .to_string()
                .contains("could not be priced")
        );
        worker.join().unwrap();
        let records: Vec<ResponseRecord> = read_jsonl(&output).unwrap();
        assert_eq!(records[0].output, "retained answer");
        assert!(records[0].execution.as_ref().unwrap().reservation_retained);
    }
}
