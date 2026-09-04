//! Deterministic stratified sampling with a private hidden-set seed.

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    BenchmarkCase, BibleReference, CaseStratum, CorpusLock, PromptVariant, ReferenceRecord,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplingConfig {
    pub schema_version: u16,
    pub release_id: String,
    pub seed: String,
    pub total_references: usize,
    pub dev_references: usize,
    pub famous_references: usize,
    pub translation_sensitive_references: usize,
    pub short_references: usize,
    pub long_references: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratedReference {
    pub reference: BibleReference,
    pub stratum: CaseStratum,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub schema_version: u16,
    pub release_id: String,
    pub public_seed: String,
    pub translations: Vec<String>,
    pub corpus_sha256: BTreeMap<String, String>,
    pub dev_reference_count: usize,
    pub dev_case_count: usize,
    pub hidden_reference_count: usize,
    pub hidden_case_count: usize,
    pub strata: BTreeMap<String, usize>,
    pub dev_cases_sha256: String,
    pub hidden_cases_sha256: String,
}

#[derive(Debug)]
pub struct SampledDataset {
    pub dev_cases: Vec<BenchmarkCase>,
    pub dev_references: Vec<ReferenceRecord>,
    pub hidden_cases: Vec<BenchmarkCase>,
    pub hidden_references: Vec<ReferenceRecord>,
    pub manifest: ReleaseManifest,
}

/// Produces deterministic, shared-reference development and hidden datasets.
///
/// # Errors
///
/// Returns an error for invalid quotas, inconsistent corpus locks, insufficient
/// common verses, or curated references missing from any translation.
pub fn sample_dataset(
    config: &SamplingConfig,
    translations: &[String],
    references: &[ReferenceRecord],
    locks: &[CorpusLock],
    curated: &[CuratedReference],
    hidden_seed: &str,
) -> Result<SampledDataset> {
    validate_config(config, translations, locks)?;
    if hidden_seed.trim().is_empty() {
        bail!("hidden sampling seed must not be empty");
    }
    let universe = common_universe(translations, references);
    if universe.len() < config.total_references {
        bail!(
            "only {} single-verse references are shared by all translations; {} requested",
            universe.len(),
            config.total_references
        );
    }

    let dev_config = partition_config(config, config.dev_references, true);
    let mut dev: HashMap<BibleReference, CaseStratum> = HashMap::new();
    select_partition(&dev_config, curated, &universe, &mut dev)?;

    let hidden_config = partition_config(
        config,
        config.total_references - config.dev_references,
        false,
    );
    let hidden_universe: HashMap<_, _> = universe
        .iter()
        .filter(|(reference, _)| !dev.contains_key(*reference))
        .map(|(reference, records)| (reference.clone(), records.clone()))
        .collect();
    let mut hidden: HashMap<BibleReference, CaseStratum> = HashMap::new();
    let mut private_config = hidden_config;
    hidden_seed.clone_into(&mut private_config.seed);
    select_partition(&private_config, curated, &hidden_universe, &mut hidden)?;

    let mut selected = dev.clone();
    selected.extend(hidden.clone());
    let dev_cases = build_cases(&config.release_id, "DEV", translations, &dev);
    let hidden_cases = build_cases(&config.release_id, "HID", translations, &hidden);
    let dev_references = select_records(references, &dev);
    let hidden_references = select_records(references, &hidden);
    let strata = stratum_counts(&selected);
    let corpus_sha256 = locks
        .iter()
        .map(|lock| (lock.translation.clone(), lock.corpus_sha256.clone()))
        .collect();

    Ok(SampledDataset {
        manifest: ReleaseManifest {
            schema_version: 1,
            release_id: config.release_id.clone(),
            public_seed: config.seed.clone(),
            translations: translations.to_vec(),
            corpus_sha256,
            dev_reference_count: dev.len(),
            dev_case_count: dev_cases.len(),
            hidden_reference_count: hidden.len(),
            hidden_case_count: hidden_cases.len(),
            strata,
            dev_cases_sha256: digest_jsonl(&dev_cases),
            hidden_cases_sha256: digest_jsonl(&hidden_cases),
        },
        dev_cases,
        dev_references,
        hidden_cases,
        hidden_references,
    })
}

fn select_partition(
    config: &SamplingConfig,
    curated: &[CuratedReference],
    universe: &HashMap<BibleReference, Vec<&ReferenceRecord>>,
    selected: &mut HashMap<BibleReference, CaseStratum>,
) -> Result<()> {
    select_curated(config, curated, universe, selected)?;
    select_translation_sensitive(config, universe, selected);
    select_by_length(
        config.short_references,
        false,
        &config.seed,
        universe,
        selected,
        CaseStratum::ShortVerse,
    );
    select_by_length(
        config.long_references,
        true,
        &config.seed,
        universe,
        selected,
        CaseStratum::LongVerse,
    );
    select_random(config, universe, selected);
    Ok(())
}

fn partition_config(config: &SamplingConfig, total: usize, public: bool) -> SamplingConfig {
    let quota = |value: usize| {
        let public_count = value * config.dev_references / config.total_references;
        if public {
            public_count
        } else {
            value - public_count
        }
    };
    SamplingConfig {
        schema_version: config.schema_version,
        release_id: config.release_id.clone(),
        seed: config.seed.clone(),
        total_references: total,
        dev_references: 0,
        famous_references: quota(config.famous_references),
        translation_sensitive_references: quota(config.translation_sensitive_references),
        short_references: quota(config.short_references),
        long_references: quota(config.long_references),
    }
}

fn validate_config(
    config: &SamplingConfig,
    translations: &[String],
    locks: &[CorpusLock],
) -> Result<()> {
    if config.schema_version != 1 {
        bail!(
            "unsupported sampling schema_version {}",
            config.schema_version
        );
    }
    if config.dev_references >= config.total_references {
        bail!("dev_references must be smaller than total_references");
    }
    let fixed = config.famous_references
        + config.translation_sensitive_references
        + config.short_references
        + config.long_references;
    if fixed > config.total_references {
        bail!("stratum quotas exceed total_references");
    }
    if translations.len() < 2 {
        bail!("sampling requires at least two translations");
    }
    let lock_ids: HashSet<&str> = locks.iter().map(|lock| lock.translation.as_str()).collect();
    for translation in translations {
        if !lock_ids.contains(translation.as_str()) {
            bail!("missing corpus lock for {translation}");
        }
    }
    Ok(())
}

fn common_universe<'a>(
    translations: &[String],
    references: &'a [ReferenceRecord],
) -> HashMap<BibleReference, Vec<&'a ReferenceRecord>> {
    let requested: HashSet<&str> = translations.iter().map(String::as_str).collect();
    let mut universe: HashMap<BibleReference, Vec<&ReferenceRecord>> = HashMap::new();
    for record in references {
        if requested.contains(record.translation.as_str()) && record.reference.verse_end.is_none() {
            universe
                .entry(record.reference.clone())
                .or_default()
                .push(record);
        }
    }
    universe.retain(|_, records| {
        records
            .iter()
            .map(|record| record.translation.as_str())
            .collect::<HashSet<_>>()
            .len()
            == translations.len()
    });
    universe
}

fn select_curated(
    config: &SamplingConfig,
    curated: &[CuratedReference],
    universe: &HashMap<BibleReference, Vec<&ReferenceRecord>>,
    selected: &mut HashMap<BibleReference, CaseStratum>,
) -> Result<()> {
    let mut candidates: Vec<&CuratedReference> = curated
        .iter()
        .filter(|item| {
            universe.contains_key(&item.reference)
                && (item.stratum == CaseStratum::ExtremelyFamous
                    || item.stratum == CaseStratum::WellKnown)
        })
        .collect();
    candidates.sort_by_key(|item| deterministic_key(&config.seed, "famous", &item.reference));
    if candidates.len() < config.famous_references {
        bail!("famous-reference quota exceeds curated input");
    }
    for item in candidates.into_iter().take(config.famous_references) {
        if !universe.contains_key(&item.reference) {
            bail!(
                "curated reference {} is not shared by every translation",
                item.reference
            );
        }
        selected.insert(item.reference.clone(), item.stratum);
    }
    Ok(())
}

fn select_translation_sensitive(
    config: &SamplingConfig,
    universe: &HashMap<BibleReference, Vec<&ReferenceRecord>>,
    selected: &mut HashMap<BibleReference, CaseStratum>,
) {
    let mut candidates: Vec<(&BibleReference, f64)> = universe
        .iter()
        .filter(|(reference, _)| !selected.contains_key(*reference))
        .map(|(reference, records)| (reference, disagreement(records)))
        .collect();
    candidates.sort_by(|(left_ref, left), (right_ref, right)| {
        right.total_cmp(left).then_with(|| {
            deterministic_key(&config.seed, "sensitive", left_ref).cmp(&deterministic_key(
                &config.seed,
                "sensitive",
                right_ref,
            ))
        })
    });
    for (reference, _) in candidates
        .into_iter()
        .take(config.translation_sensitive_references)
    {
        selected.insert(reference.clone(), CaseStratum::TranslationSensitive);
    }
}

fn select_by_length(
    count: usize,
    descending: bool,
    seed: &str,
    universe: &HashMap<BibleReference, Vec<&ReferenceRecord>>,
    selected: &mut HashMap<BibleReference, CaseStratum>,
    stratum: CaseStratum,
) {
    let mut candidates: Vec<(&BibleReference, usize)> = universe
        .iter()
        .filter(|(reference, _)| !selected.contains_key(*reference))
        .map(|(reference, records)| {
            let mean = records
                .iter()
                .map(|record| word_count(&record.text))
                .sum::<usize>()
                / records.len();
            (reference, mean)
        })
        .collect();
    candidates.sort_by(
        |(left_reference, left_length), (right_reference, right_length)| {
            left_length.cmp(right_length).then_with(|| {
                deterministic_key(seed, "length", left_reference).cmp(&deterministic_key(
                    seed,
                    "length",
                    right_reference,
                ))
            })
        },
    );
    if descending {
        candidates.reverse();
    }
    for (reference, _) in candidates.into_iter().take(count) {
        selected.insert(reference.clone(), stratum);
    }
}

fn select_random(
    config: &SamplingConfig,
    universe: &HashMap<BibleReference, Vec<&ReferenceRecord>>,
    selected: &mut HashMap<BibleReference, CaseStratum>,
) {
    let remaining = config.total_references - selected.len();
    let mut candidates: Vec<&BibleReference> = universe
        .keys()
        .filter(|reference| !selected.contains_key(*reference))
        .collect();
    candidates.sort_by_key(|reference| deterministic_key(&config.seed, "random", reference));
    for reference in candidates.into_iter().take(remaining) {
        selected.insert(reference.clone(), CaseStratum::Random);
    }
}

fn build_cases(
    release_id: &str,
    split: &str,
    translations: &[String],
    selected: &HashMap<BibleReference, CaseStratum>,
) -> Vec<BenchmarkCase> {
    let mut references: Vec<_> = selected.iter().collect();
    references.sort_by_key(|(reference, _)| reference.to_string());
    let mut cases = Vec::with_capacity(references.len() * translations.len());
    for (reference, stratum) in references {
        for translation in translations {
            let identity = format!("{release_id}|{split}|{translation}|{reference}");
            let digest = format!("{:x}", Sha256::digest(identity.as_bytes()));
            cases.push(BenchmarkCase {
                case_id: format!("BQ-{split}-{}", &digest[..16]).to_uppercase(),
                translation: translation.clone(),
                reference: reference.clone(),
                stratum: *stratum,
                prompt_variant: PromptVariant::Canonical,
            });
        }
    }
    cases
}

fn select_records(
    references: &[ReferenceRecord],
    selected: &HashMap<BibleReference, CaseStratum>,
) -> Vec<ReferenceRecord> {
    let mut records: Vec<_> = references
        .iter()
        .filter(|record| selected.contains_key(&record.reference))
        .cloned()
        .collect();
    records.sort_by_key(|record| (record.reference.to_string(), record.translation.clone()));
    records
}

#[allow(clippy::cast_precision_loss)]
fn disagreement(records: &[&ReferenceRecord]) -> f64 {
    let tokens: Vec<Vec<&str>> = records
        .iter()
        .map(|record| record.text.split_whitespace().collect())
        .collect();
    let mut total = 0.0;
    let mut pairs = 0_usize;
    for left in 0..tokens.len() {
        for right in (left + 1)..tokens.len() {
            let denominator = tokens[left].len().max(tokens[right].len()).max(1);
            total += levenshtein(&tokens[left], &tokens[right]) as f64 / denominator as f64;
            pairs += 1;
        }
    }
    total / pairs.max(1) as f64
}

fn levenshtein<T: Eq>(left: &[T], right: &[T]) -> usize {
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];
    for (row, left_item) in left.iter().enumerate() {
        current[0] = row + 1;
        for (column, right_item) in right.iter().enumerate() {
            current[column + 1] = (previous[column] + usize::from(left_item != right_item))
                .min(previous[column + 1] + 1)
                .min(current[column] + 1);
        }
        std::mem::swap(&mut current, &mut previous);
    }
    previous[right.len()]
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn deterministic_key(seed: &str, purpose: &str, reference: &BibleReference) -> [u8; 32] {
    Sha256::digest(format!("{seed}\0{purpose}\0{reference}").as_bytes()).into()
}

fn digest_jsonl<T: Serialize>(records: &[T]) -> String {
    let mut digest = Sha256::new();
    for record in records {
        digest.update(serde_json::to_vec(record).expect("serializing domain records cannot fail"));
        digest.update(b"\n");
    }
    format!("{:x}", digest.finalize())
}

fn stratum_counts(selected: &HashMap<BibleReference, CaseStratum>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for stratum in selected.values() {
        let name = serde_json::to_value(stratum)
            .expect("serializing enum cannot fail")
            .as_str()
            .expect("stratum serializes to string")
            .to_owned();
        *counts.entry(name).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_key_is_repeatable_and_purpose_separated() {
        let reference = BibleReference {
            book: "John".to_owned(),
            chapter: 3,
            verse_start: 16,
            verse_end: None,
        };
        assert_eq!(
            deterministic_key("seed", "a", &reference),
            deterministic_key("seed", "a", &reference)
        );
        assert_ne!(
            deterministic_key("seed", "a", &reference),
            deterministic_key("seed", "b", &reference)
        );
    }

    #[test]
    fn levenshtein_counts_word_changes() {
        assert_eq!(levenshtein(&["a", "b", "c"], &["a", "x", "c", "d"]), 2);
    }

    #[test]
    fn sampling_is_repeatable_across_fresh_hash_maps() {
        let config = SamplingConfig {
            schema_version: 1,
            release_id: "test".to_owned(),
            seed: "seed".to_owned(),
            total_references: 6,
            dev_references: 2,
            famous_references: 0,
            translation_sensitive_references: 1,
            short_references: 1,
            long_references: 1,
        };
        let translations = vec!["a".to_owned(), "b".to_owned()];
        let mut references = Vec::new();
        for verse in 1..=10 {
            for translation in &translations {
                references.push(ReferenceRecord {
                    translation: translation.clone(),
                    reference: BibleReference {
                        book: "Genesis".to_owned(),
                        chapter: 1,
                        verse_start: verse,
                        verse_end: None,
                    },
                    text: format!("{translation} text with {verse} words"),
                });
            }
        }
        let locks: Vec<CorpusLock> = translations
            .iter()
            .map(|translation| CorpusLock {
                schema_version: 1,
                translation: translation.clone(),
                edition: "test".to_owned(),
                source_url: "https://example.test".to_owned(),
                source_sha256: "source".to_owned(),
                importer_version: "test".to_owned(),
                artifacts: vec![],
                reference_count: 10,
                corpus_sha256: format!("corpus-{translation}"),
            })
            .collect();
        let first = sample_dataset(
            &config,
            &translations,
            &references,
            &locks,
            &[],
            "private-seed",
        )
        .unwrap();
        let second = sample_dataset(
            &config,
            &translations,
            &references,
            &locks,
            &[],
            "private-seed",
        )
        .unwrap();
        let different_private_seed = sample_dataset(
            &config,
            &translations,
            &references,
            &locks,
            &[],
            "different-private-seed",
        )
        .unwrap();
        assert_eq!(
            first.manifest.dev_cases_sha256,
            second.manifest.dev_cases_sha256
        );
        assert_eq!(
            first.manifest.hidden_cases_sha256,
            second.manifest.hidden_cases_sha256
        );
        assert_eq!(
            first.manifest.dev_cases_sha256,
            different_private_seed.manifest.dev_cases_sha256
        );
        assert_ne!(
            first.manifest.hidden_cases_sha256,
            different_private_seed.manifest.hidden_cases_sha256
        );
    }
}
