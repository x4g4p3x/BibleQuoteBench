//! Public diagnostic datasets and an explicitly synthetic, zero-network release pilot.

use crate::{
    BenchmarkCase, CaseStratum, LicenseKind, PromptVariant, ReferenceRecord, ResponseRecord,
    TranslationCatalog,
};
use crate::{
    io::{read_json, read_jsonl, write_json, write_jsonl, write_text},
    provider::{ProviderConfig, ProviderKind},
    study::{analyze, make_manifest, manifest_path, write_study},
};
use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

pub const TRACKS: [&str; 5] = [
    "canonical",
    "concise",
    "word_for_word",
    "copy_control",
    "passage",
];

/// Prepares shared public anchors across prompt and copy controls, plus disjoint three-verse passages.
///
/// # Errors
/// Rejects private data, insufficient anchors, or missing contiguous corpus coverage.
#[allow(clippy::too_many_lines)]
pub fn prepare(
    cases: &[BenchmarkCase],
    corpus: &[ReferenceRecord],
    catalog: &TranslationCatalog,
    count: usize,
    output: &Path,
) -> Result<()> {
    crate::validate_dataset(catalog, cases, corpus)?;
    if count < 2
        || catalog.translations.iter().any(|spec| {
            spec.license_kind != LicenseKind::PublicDomain || !spec.redistribute_reference_text
        })
        || cases
            .iter()
            .any(|case| !case.case_id.starts_with("BQ-DEV-"))
    {
        bail!(
            "pilot requires at least two public development references and redistributable editions"
        );
    }
    let mut strata: BTreeMap<String, BTreeMap<String, &crate::BibleReference>> = BTreeMap::new();
    for case in cases {
        strata
            .entry(format!("{:?}", case.stratum))
            .or_default()
            .insert(case.reference.to_string(), &case.reference);
    }
    let mut pools: Vec<_> = strata
        .values()
        .map(|values| values.values().copied().collect::<Vec<_>>().into_iter())
        .collect();
    let mut anchors = BTreeSet::new();
    loop {
        let previous = anchors.len();
        for pool in &mut pools {
            if anchors.len() < count {
                if let Some(reference) = pool.next() {
                    anchors.insert(reference.to_string());
                }
            }
        }
        if anchors.len() == count {
            break;
        }
        if anchors.len() == previous {
            bail!("not enough public pilot references");
        }
    }
    let base: Vec<_> = cases
        .iter()
        .filter(|case| anchors.contains(&case.reference.to_string()))
        .cloned()
        .collect();
    for (name, variant) in [
        ("canonical", PromptVariant::Canonical),
        ("concise", PromptVariant::Concise),
        ("word_for_word", PromptVariant::WordForWord),
        ("copy_control", PromptVariant::CopyControl),
    ] {
        let selected: Vec<_> = base
            .iter()
            .map(|case| {
                let mut case = case.clone();
                case.prompt_variant = variant;
                case.case_id = format!("{}-{name}", case.case_id);
                case
            })
            .collect();
        let references: Vec<_> = corpus
            .iter()
            .filter(|record| {
                base.iter().any(|case| {
                    case.reference == record.reference && case.translation == record.translation
                })
            })
            .cloned()
            .collect();
        write_dataset(&output.join(name), &selected, &references, catalog)?;
    }
    let mut passage_cases = Vec::new();
    let mut passage_references = Vec::new();
    let mut occupied = BTreeSet::new();
    let mut candidates: BTreeMap<String, Vec<&BenchmarkCase>> = BTreeMap::new();
    for case in cases {
        candidates
            .entry(case.reference.to_string())
            .or_default()
            .push(case);
    }
    let mut built = 0;
    for group in candidates.values() {
        if built == count {
            break;
        }
        let start = &group[0].reference;
        let Some(end) = start.verse_start.checked_add(2) else {
            continue;
        };
        if (start.verse_start..=end)
            .any(|verse| occupied.contains(&(start.book.clone(), start.chapter, verse)))
        {
            continue;
        }
        let mut texts = Vec::new();
        for case in group {
            let pieces: Option<Vec<_>> = (start.verse_start..=end)
                .map(|verse| {
                    corpus
                        .iter()
                        .find(|record| {
                            record.translation == case.translation
                                && record.reference.book == start.book
                                && record.reference.chapter == start.chapter
                                && record.reference.verse_start == verse
                                && record.reference.verse_end.is_none()
                        })
                        .map(|record| record.text.clone())
                })
                .collect();
            if let Some(pieces) = pieces {
                texts.push(pieces.join(" "));
            }
        }
        if texts.len() != catalog.translations.len() {
            continue;
        }
        for (case, text) in group.iter().zip(texts) {
            let mut case = (*case).clone();
            case.reference.verse_end = Some(end);
            case.stratum = CaseStratum::Passage;
            case.prompt_variant = PromptVariant::Canonical;
            case.case_id = format!("{}-passage", case.case_id);
            passage_references.push(ReferenceRecord {
                translation: case.translation.clone(),
                reference: case.reference.clone(),
                text,
            });
            passage_cases.push(case);
        }
        for verse in start.verse_start..=end {
            occupied.insert((start.book.clone(), start.chapter, verse));
        }
        built += 1;
    }
    if built != count {
        bail!("insufficient nonoverlapping three-verse passages with coverage in every edition");
    }
    write_dataset(
        &output.join("passage"),
        &passage_cases,
        &passage_references,
        catalog,
    )?;
    write_json(
        &output.join("live-plan.json"),
        &serde_json::json!({
            "schema_version": 2, "execution_enabled": false, "status": "held_by_user", "provider": "openai",
            "requested_model_label": "GPT-6 Astra (max reasoning)", "verified_api_model_id": "gpt-6-astra", "model_documentation_url": "https://developers.openai.com/api/docs/models/gpt-6-astra", "model_documentation_checked": "2026-09-05", "documented_reasoning_levels": ["low", "medium", "high", "xhigh", "max"], "requested_reasoning_verified": true, "temperature": null,
            "requested_reasoning_effort": "max", "budget_eur": 20, "spent_eur": 0,
            "reference_count_per_track": count, "tracks": TRACKS, "repetitions": 3,
            "planned_requests": count * catalog.translations.len() * TRACKS.len() * 3,
            "requirements_before_execution": ["Explicit user authorization to spend", "Verify account access for gpt-6-astra", "Configure credential locally", "Estimate costs using verified pricing and exchange rate; enforce remaining budget before each request"]
        }),
    )?;
    write_text(
        &output.join("README.md"),
        "# BibleQuoteBench v0.2 pilot datasets\n\nFive separate tracks use official, locked public-domain text. Canonical, concise, word-for-word, and copy-control tracks share anchors. The passage track selects nonoverlapping three-verse spans from public development anchors with complete corpus coverage. Verse texts are joined by one ASCII space, without verse numbers.\n\nThe copy control explicitly supplies the answer and must never enter the closed-book score. Single-verse anchors are selected round-robin across existing strata; the pilot is descriptive, not representative of the full release mixture.\n\nThe live plan records the requested GPT-6 Astra (max reasoning) target and EUR 20 ceiling. Execution is disabled at the user's request. This plan is not a spending authorization or an implemented billing cap. No live model results are included.\n",
    )
}

fn write_dataset(
    path: &Path,
    cases: &[BenchmarkCase],
    references: &[ReferenceRecord],
    catalog: &TranslationCatalog,
) -> Result<()> {
    crate::validate_dataset(catalog, cases, references)?;
    std::fs::create_dir_all(path)?;
    write_jsonl(Some(&path.join("cases.jsonl")), cases)?;
    write_jsonl(Some(&path.join("references.jsonl")), references)?;
    write_json(&path.join("translations.json"), catalog)
}

/// Runs deterministic synthetic response generators through the real analysis pipeline.
/// No network or provider execution is used; synthetic provenance is retained in every manifest.
///
/// # Errors
/// Returns dataset, scoring, or artifact errors.
pub fn synthetic(dataset_root: &Path, output: &Path) -> Result<()> {
    std::fs::create_dir_all(output)?;
    let mut summary = String::from(
        "# BibleQuoteBench v0.2 synthetic pilot\n\n**Synthetic validation only. These are constructed responses, not measurements of GPT-6 or any live model. No paid calls were made.**\n\nThe two synthetic generators deliberately exercise exact recall, punctuation changes, edition substitution, mixed wording, refusal, empty output, and provider failure. Their names and rates have no model-performance meaning. Three repetitions per generator are evaluated with complete coverage.\n\n| Track | Cases/run | Reference clusters | Synthetic A ExactText | Synthetic B ExactText |\n| --- | ---: | ---: | ---: | ---: |\n",
    );
    let mut study_reports = Vec::new();
    for name in TRACKS {
        let root = dataset_root.join(name);
        let cases: Vec<BenchmarkCase> = read_jsonl(&root.join("cases.jsonl"))?;
        let references: Vec<ReferenceRecord> = read_jsonl(&root.join("references.jsonl"))?;
        let catalog: TranslationCatalog = read_json(&root.join("translations.json"))?;
        let target = output.join(name);
        std::fs::create_dir_all(&target)?;
        let mut inputs = Vec::new();
        for model in ["synthetic-a", "synthetic-b"] {
            for repeat in 0..3 {
                let config = ProviderConfig {
                    kind: ProviderKind::OpenaiCompatible,
                    model: model.into(),
                    run_id: format!("{model}-{repeat}"),
                    api_key_env: None,
                    base_url: Some("http://synthetic.invalid".into()),
                    temperature: Some(0.0),
                    reasoning_effort: None,
                    max_output_tokens: 512,
                    case_limit: None,
                    fail_fast: false,
                };
                let responses: Vec<_> = cases
                    .iter()
                    .enumerate()
                    .map(|(index, case)| {
                        synthetic_response(&config, case, &references, index, repeat)
                    })
                    .collect();
                let mut manifest =
                    make_manifest(&config, &cases, &references, &catalog, &responses)?;
                manifest.evidence = "synthetic_fixture".into();
                let path = target.join(format!("{}.jsonl", config.run_id));
                write_jsonl(Some(&path), &responses)?;
                write_json(&manifest_path(&path), &manifest)?;
                inputs.push((manifest, responses));
            }
        }
        let report = analyze(&cases, &references, &catalog, &inputs, 2000)?;
        let values: Vec<_> = report.models.values().collect();
        let _ = writeln!(
            summary,
            "| [{name}]({name}/analysis.md) | {} | {} | {:.2}% | {:.2}% |",
            cases.len(),
            values[0].exact_text.clusters,
            values[0].exact_text.estimate * 100.0,
            values[1].exact_text.estimate * 100.0
        );
        write_study(&target, &report)?;
        study_reports.push(report);
    }
    crate::visualization::write_html(&output.join("index.html"), &study_reports)?;
    summary.push_str("\n[Open the interactive results report](index.html) for model comparisons, edition diagnostics, and annotated failure exploration. It is self-contained and works offline.\n");
    summary.push_str("\nEach linked analysis includes cluster-bootstrap intervals, paired differences, edition-differing subsets, translation-by-stratum breakdowns, provider-error accounting, and annotated failures. Raw synthetic responses and manifest hashes permit independent reconstruction.\n\nLive pilot: prepared but held by user. Requested target: GPT-6 Astra (max reasoning); budget ceiling: EUR 20; spent: EUR 0. The documented API model is gpt-6-astra; the requested max reasoning level is documented. Account access, current pricing, and an explicitly enabled budget policy must be verified before any later live execution.\n");
    write_text(&output.join("README.md"), &summary)
}

fn synthetic_response(
    config: &ProviderConfig,
    case: &BenchmarkCase,
    references: &[ReferenceRecord],
    index: usize,
    repeat: usize,
) -> ResponseRecord {
    let expected = references
        .iter()
        .find(|record| record.translation == case.translation && record.reference == case.reference)
        .expect("validated references");
    let alternative = references.iter().find(|record| {
        record.translation != case.translation
            && record.reference == case.reference
            && record.text != expected.text
    });
    let category = (index + repeat + usize::from(config.model == "synthetic-b") * 3) % 11;
    let mut error = None;
    let output = if case.prompt_variant == PromptVariant::CopyControl && category != 8 {
        expected.text.clone()
    } else {
        match category {
            0..=3 => expected.text.clone(),
            4 => expected.text.replace([',', '.', ';', ':', '!', '?'], ""),
            5 => alternative.unwrap_or(expected).text.clone(),
            6 => {
                let words: Vec<_> = expected.text.split_whitespace().collect();
                let other: Vec<_> = alternative
                    .unwrap_or(expected)
                    .text
                    .split_whitespace()
                    .collect();
                format!(
                    "{} {}",
                    words[..words.len() / 2].join(" "),
                    other[other.len() / 2..].join(" ")
                )
            }
            7 => "I cannot provide that passage.".into(),
            8 => format!("Here is the passage: {}", expected.text),
            9 => String::new(),
            _ => {
                error = Some("synthetic provider failure".into());
                String::new()
            }
        }
    };
    ResponseRecord {
        case_id: case.case_id.clone(),
        run_id: config.run_id.clone(),
        provider: config.kind.name().into(),
        model: config.model.clone(),
        resolved_model: Some(config.model.clone()),
        output,
        error,
        temperature: config.temperature,
        reasoning_effort: config.reasoning_effort.clone(),
        seed: None,
        provider_request_id: None,
        system_fingerprint: None,
        execution: None,
    }
}
