//! Cross-file schema, licensing, reference, and corpus integrity validation.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};

use crate::{BenchmarkCase, LicenseKind, ReferenceRecord, TranslationCatalog};

/// Validates cross-file dataset integrity and licensing invariants.
///
/// # Errors
///
/// Returns an error for unsupported schemas, duplicate identifiers, malformed
/// references, missing corpus coverage, or unsafe licensed-text metadata.
pub fn validate_dataset(
    catalog: &TranslationCatalog,
    cases: &[BenchmarkCase],
    references: &[ReferenceRecord],
) -> Result<()> {
    if catalog.schema_version != 1 {
        bail!(
            "unsupported translation catalog schema_version {}",
            catalog.schema_version
        );
    }
    if catalog.translations.is_empty() {
        bail!("translation catalog must contain at least one translation");
    }
    if cases.is_empty() {
        bail!("dataset must contain at least one case");
    }
    if references.is_empty() {
        bail!("corpus must contain at least one reference record");
    }

    let translation_ids = validate_catalog(catalog)?;
    let reference_keys = validate_references(references, &translation_ids)?;
    validate_cases(cases, &translation_ids, &reference_keys)
}

fn validate_catalog(catalog: &TranslationCatalog) -> Result<HashSet<&str>> {
    let mut translation_ids = HashSet::new();
    for translation in &catalog.translations {
        if translation.id.trim().is_empty() {
            bail!("translation id must not be empty");
        }
        if !translation_ids.insert(translation.id.as_str()) {
            bail!("duplicate translation id: {}", translation.id);
        }
        if translation.edition.trim().is_empty() {
            bail!("translation {} has no pinned edition", translation.id);
        }
        if translation.license_url.trim().is_empty() || translation.source_url.trim().is_empty() {
            bail!(
                "translation {} must declare license_url and source_url",
                translation.id
            );
        }
        if translation.license_kind == LicenseKind::LicensedPrivate
            && translation.redistribute_reference_text
        {
            bail!(
                "licensed-private translation {} cannot enable reference-text redistribution",
                translation.id
            );
        }
    }
    Ok(translation_ids)
}

fn validate_references<'a>(
    references: &'a [ReferenceRecord],
    translation_ids: &HashSet<&str>,
) -> Result<HashSet<(&'a str, &'a crate::BibleReference)>> {
    let mut reference_keys = HashSet::new();
    for record in references {
        record
            .reference
            .validate()
            .map_err(|error| anyhow::anyhow!("invalid corpus reference: {error}"))?;
        if !translation_ids.contains(record.translation.as_str()) {
            bail!("corpus uses unknown translation: {}", record.translation);
        }
        if record.text.is_empty() {
            bail!(
                "empty reference text for {} in {}",
                record.reference,
                record.translation
            );
        }
        if record.text.trim() != record.text {
            bail!(
                "reference text has leading or trailing whitespace for {} in {}",
                record.reference,
                record.translation
            );
        }
        if record.text.contains('\\')
            || ["strong=", "lemma=", "x-morph=", "x-occurrence="]
                .iter()
                .any(|attribute| record.text.contains(attribute))
        {
            bail!(
                "reference text contains residual USFM markup for {} in {}",
                record.reference,
                record.translation
            );
        }
        let key = (record.translation.as_str(), &record.reference);
        if !reference_keys.insert(key) {
            bail!(
                "duplicate corpus record for {} in {}",
                record.reference,
                record.translation
            );
        }
    }
    Ok(reference_keys)
}

fn validate_cases(
    cases: &[BenchmarkCase],
    translation_ids: &HashSet<&str>,
    reference_keys: &HashSet<(&str, &crate::BibleReference)>,
) -> Result<()> {
    let mut case_ids = HashSet::new();
    let mut cases_per_reference: HashMap<&crate::BibleReference, HashSet<&str>> = HashMap::new();
    for case in cases {
        if case.case_id.trim().is_empty() {
            bail!("case_id must not be empty");
        }
        if !case_ids.insert(case.case_id.as_str()) {
            bail!("duplicate case_id: {}", case.case_id);
        }
        case.reference
            .validate()
            .map_err(|error| anyhow::anyhow!("invalid case {}: {error}", case.case_id))?;
        if !translation_ids.contains(case.translation.as_str()) {
            bail!(
                "case {} uses unknown translation: {}",
                case.case_id,
                case.translation
            );
        }
        if !reference_keys.contains(&(case.translation.as_str(), &case.reference)) {
            bail!(
                "case {} has no reference text for {} in {}",
                case.case_id,
                case.reference,
                case.translation
            );
        }
        let seen_translations = cases_per_reference.entry(&case.reference).or_default();
        if !seen_translations.insert(case.translation.as_str()) {
            bail!(
                "multiple cases request {} in {}",
                case.reference,
                case.translation
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{BibleReference, CaseStratum, LicenseKind, PromptVariant, TranslationSpec};

    use super::*;

    fn fixture() -> (TranslationCatalog, Vec<BenchmarkCase>, Vec<ReferenceRecord>) {
        let reference = BibleReference {
            book: "John".to_owned(),
            chapter: 3,
            verse_start: 16,
            verse_end: None,
        };
        (
            TranslationCatalog {
                schema_version: 1,
                translations: vec![TranslationSpec {
                    id: "test-1".to_owned(),
                    name: "Test".to_owned(),
                    abbreviation: "TST".to_owned(),
                    edition: "1".to_owned(),
                    license_kind: LicenseKind::PublicDomain,
                    license_url: "https://example.test/license".to_owned(),
                    source_url: "https://example.test/source".to_owned(),
                    redistribute_reference_text: true,
                }],
            },
            vec![BenchmarkCase {
                case_id: "BQ-1".to_owned(),
                translation: "test-1".to_owned(),
                reference: reference.clone(),
                stratum: CaseStratum::ExtremelyFamous,
                prompt_variant: PromptVariant::Canonical,
            }],
            vec![ReferenceRecord {
                translation: "test-1".to_owned(),
                reference,
                text: "Reference text.".to_owned(),
            }],
        )
    }

    #[test]
    fn accepts_complete_dataset() {
        let (catalog, cases, references) = fixture();
        validate_dataset(&catalog, &cases, &references).unwrap();
    }

    #[test]
    fn rejects_case_without_reference_text() {
        let (catalog, cases, _) = fixture();
        let error = validate_dataset(&catalog, &cases, &[]).unwrap_err();
        assert!(error.to_string().contains("at least one reference"));
    }

    #[test]
    fn rejects_private_text_marked_for_redistribution() {
        let (mut catalog, cases, references) = fixture();
        catalog.translations[0].license_kind = LicenseKind::LicensedPrivate;
        let error = validate_dataset(&catalog, &cases, &references).unwrap_err();
        assert!(error.to_string().contains("cannot enable"));
    }

    #[test]
    fn rejects_residual_usfm_markup() {
        let (catalog, cases, mut references) = fixture();
        references[0].text = r"Reference \w text".to_owned();
        let error = validate_dataset(&catalog, &cases, &references).unwrap_err();
        assert!(error.to_string().contains("residual USFM markup"));
    }
}
