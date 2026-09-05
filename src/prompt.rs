//! Deterministic, edition-pinned closed-book prompt rendering.

use crate::{BenchmarkCase, PromptVariant, TranslationSpec};

/// Renders the selected standardized prompt for one benchmark case.
pub fn render_prompt(case: &BenchmarkCase, translation: &TranslationSpec) -> String {
    let source = format!("{} ({})", translation.name, translation.edition);
    match case.prompt_variant {
        PromptVariant::Canonical => format!(
            "Quote {} from the {source}.\nGive the passage exactly as it appears in that translation.\nOutput only the passage text.",
            case.reference
        ),
        PromptVariant::Concise => {
            format!(
                "Quote {} in the {source}. Output only the passage text.",
                case.reference
            )
        }
        PromptVariant::WordForWord => format!(
            "Reproduce {} word-for-word according to the {source}.\nOutput only the passage text.",
            case.reference
        ),
        PromptVariant::CopyControl => format!(
            "Copy the supplied text for {} from the {source} exactly.\nOutput only the passage text.",
            case.reference
        ),
    }
}

/// Renders an executable prompt, requiring supplied text only for copy controls.
///
/// # Errors
/// Returns an error if a copy control lacks its explicitly supplied text.
pub fn execution_prompt(
    case: &BenchmarkCase,
    translation: &TranslationSpec,
    supplied: Option<&str>,
) -> anyhow::Result<String> {
    let mut prompt = render_prompt(case, translation);
    if case.prompt_variant == crate::PromptVariant::CopyControl {
        let text =
            supplied.ok_or_else(|| anyhow::anyhow!("copy control requires supplied text"))?;
        prompt.push_str("\n\n<supplied_text>\n");
        prompt.push_str(text);
        prompt.push_str("\n</supplied_text>");
    }
    Ok(prompt)
}

#[cfg(test)]
mod tests {
    use crate::{BibleReference, CaseStratum, LicenseKind};

    use super::*;

    #[test]
    fn canonical_prompt_pins_edition() {
        let case = BenchmarkCase {
            case_id: "case".to_owned(),
            translation: "web-2020".to_owned(),
            reference: BibleReference {
                book: "John".to_owned(),
                chapter: 3,
                verse_start: 16,
                verse_end: None,
            },
            stratum: CaseStratum::ExtremelyFamous,
            prompt_variant: PromptVariant::Canonical,
        };
        let translation = TranslationSpec {
            id: "web-2020".to_owned(),
            name: "World English Bible".to_owned(),
            abbreviation: "WEB".to_owned(),
            edition: "2020 stable text edition".to_owned(),
            license_kind: LicenseKind::PublicDomain,
            license_url: "https://example.test/license".to_owned(),
            source_url: "https://example.test/source".to_owned(),
            redistribute_reference_text: true,
        };

        assert_eq!(
            render_prompt(&case, &translation),
            "Quote John 3:16 from the World English Bible (2020 stable text edition).\n\
             Give the passage exactly as it appears in that translation.\n\
             Output only the passage text."
        );
        let copy = BenchmarkCase {
            prompt_variant: PromptVariant::CopyControl,
            ..case.clone()
        };
        assert!(execution_prompt(&copy, &translation, None).is_err());
        let copy_prompt =
            execution_prompt(&copy, &translation, Some("unique supplied passage")).unwrap();
        assert!(copy_prompt.contains("<supplied_text>\nunique supplied passage\n</supplied_text>"));
        for variant in [
            PromptVariant::Canonical,
            PromptVariant::Concise,
            PromptVariant::WordForWord,
        ] {
            let recall = BenchmarkCase {
                prompt_variant: variant,
                ..case.clone()
            };
            assert!(
                !execution_prompt(&recall, &translation, Some("unique supplied passage"))
                    .unwrap()
                    .contains("unique supplied passage")
            );
        }
    }
}
