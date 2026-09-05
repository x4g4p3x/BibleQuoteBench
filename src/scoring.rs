//! Normative exact-match, edit-distance, and translation-confusion scoring.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::importer::sha256_hex;
use crate::{BenchmarkCase, Classification, ReferenceRecord, ResponseRecord, ScoreRecord};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct EditCounts {
    insertions: usize,
    deletions: usize,
    substitutions: usize,
}

impl EditCounts {
    const fn total(self) -> usize {
        self.insertions + self.deletions + self.substitutions
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreSummary {
    pub responses: usize,
    pub exact_text_rate: f64,
    pub exact_words_rate: f64,
    pub mean_word_accuracy: f64,
    pub mean_word_error_rate: f64,
    pub mean_character_error_rate: f64,
    pub refusal_rate: f64,
    pub provider_error_rate: f64,
    pub extraneous_text_rate: f64,
    pub translation_confusion_rate: f64,
    pub classifications: BTreeMap<String, usize>,
}

/// Scores one retained provider response against its requested and alternative editions.
pub fn score_response(
    case: &BenchmarkCase,
    response: &ResponseRecord,
    requested: &ReferenceRecord,
    alternatives: &[&ReferenceRecord],
) -> ScoreRecord {
    let expected_text = normalize_technical(&requested.text);
    let output_text = normalize_technical(&response.output);
    let expected_words = tokenize_words(&expected_text);
    let output_words = tokenize_words(&output_text);
    let word_edits = edit_counts(&expected_words, &output_words);
    let expected_chars: Vec<char> = expected_text.chars().collect();
    let output_chars: Vec<char> = output_text.chars().collect();
    let character_edits = edit_counts(&expected_chars, &output_chars).total();

    let exact_text = response.error.is_none() && output_text == expected_text;
    let exact_words = response.error.is_none() && output_words == expected_words;
    let refusal = looks_like_refusal(&output_text);
    let extraneous_text = !exact_text
        && ((!expected_text.is_empty() && output_text.contains(&expected_text))
            || (output_words.len() > expected_words.len()
                && contains_contiguous(&output_words, &expected_words)));

    let mut exact_other_translations: Vec<_> = alternatives
        .iter()
        .filter(|alternative| normalize_technical(&alternative.text) == output_text)
        .map(|alternative| alternative.translation.clone())
        .collect();
    exact_other_translations.sort();
    exact_other_translations.dedup();
    let exact_other_translation = exact_other_translations.first().cloned();

    let mut closest_translations = Vec::new();
    let mut closest_distance = word_edits.total();
    for alternative in alternatives {
        let alternative_words = tokenize_words(&normalize_technical(&alternative.text));
        let distance = edit_counts(&alternative_words, &output_words).total();
        if distance < closest_distance {
            closest_distance = distance;
            closest_translations = vec![alternative.translation.clone()];
        } else if distance == closest_distance && distance < word_edits.total() {
            closest_translations.push(alternative.translation.clone());
        }
    }
    closest_translations.sort();
    closest_translations.dedup();
    if output_words.is_empty() || refusal || response.error.is_some() {
        closest_translations.clear();
    }
    let closest_translation = closest_translations.first().cloned();

    let translation_contamination_rate = contamination_rate(
        &expected_words,
        &output_words,
        alternatives
            .iter()
            .map(|alternative| tokenize_words(&normalize_technical(&alternative.text)))
            .collect::<Vec<_>>()
            .as_slice(),
    );

    let classification = if response.error.is_some() {
        Classification::ProviderError
    } else if output_text.is_empty() {
        Classification::Empty
    } else if refusal {
        Classification::Refusal
    } else if exact_text {
        Classification::ExactRequested
    } else if exact_other_translation.is_some() {
        Classification::TranslationConfusion
    } else if extraneous_text {
        Classification::ExtraneousText
    } else {
        Classification::Partial
    };

    let word_error_rate = rate(word_edits.total(), expected_words.len());
    let character_error_rate = rate(character_edits, expected_chars.len());

    ScoreRecord {
        case_id: case.case_id.clone(),
        run_id: response.run_id.clone(),
        provider: response.provider.clone(),
        model: response.model.clone(),
        resolved_model: response.resolved_model.clone(),
        response_sha256: sha256_hex(response.output.as_bytes()),
        requested_translation: case.translation.clone(),
        reference: case.reference.clone(),
        stratum: case.stratum,
        exact_text,
        exact_words,
        word_error_rate,
        character_error_rate,
        word_accuracy: (1.0 - word_error_rate).max(0.0),
        insertions: word_edits.insertions,
        deletions: word_edits.deletions,
        substitutions: word_edits.substitutions,
        refusal,
        extraneous_text,
        classification,
        exact_other_translation,
        exact_other_translations,
        closest_translation,
        closest_translations,
        translation_contamination_rate,
    }
}

/// Aggregates individual score records into benchmark-level rates and means.
pub fn aggregate_scores(scores: &[ScoreRecord]) -> ScoreSummary {
    if scores.is_empty() {
        return ScoreSummary {
            responses: 0,
            exact_text_rate: 0.0,
            exact_words_rate: 0.0,
            mean_word_accuracy: 0.0,
            mean_word_error_rate: 0.0,
            mean_character_error_rate: 0.0,
            refusal_rate: 0.0,
            provider_error_rate: 0.0,
            extraneous_text_rate: 0.0,
            translation_confusion_rate: 0.0,
            classifications: BTreeMap::new(),
        };
    }

    #[allow(clippy::cast_precision_loss)]
    let count = scores.len() as f64;
    let mut classifications = BTreeMap::new();
    for score in scores {
        let key = classification_name(score.classification).to_owned();
        *classifications.entry(key).or_insert(0) += 1;
    }

    ScoreSummary {
        responses: scores.len(),
        exact_text_rate: ratio(
            scores.iter().filter(|score| score.exact_text).count(),
            scores.len(),
        ),
        exact_words_rate: ratio(
            scores.iter().filter(|score| score.exact_words).count(),
            scores.len(),
        ),
        mean_word_accuracy: scores.iter().map(|score| score.word_accuracy).sum::<f64>() / count,
        mean_word_error_rate: scores
            .iter()
            .map(|score| score.word_error_rate)
            .sum::<f64>()
            / count,
        mean_character_error_rate: scores
            .iter()
            .map(|score| score.character_error_rate)
            .sum::<f64>()
            / count,
        refusal_rate: ratio(
            scores.iter().filter(|score| score.refusal).count(),
            scores.len(),
        ),
        provider_error_rate: ratio(
            scores
                .iter()
                .filter(|score| score.classification == Classification::ProviderError)
                .count(),
            scores.len(),
        ),
        extraneous_text_rate: ratio(
            scores.iter().filter(|score| score.extraneous_text).count(),
            scores.len(),
        ),
        translation_confusion_rate: ratio(
            scores
                .iter()
                .filter(|score| score.classification == Classification::TranslationConfusion)
                .count(),
            scores.len(),
        ),
        classifications,
    }
}

fn normalize_technical(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .nfc()
        .collect()
}

fn tokenize_words(text: &str) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut current = String::new();

    for (index, &character) in characters.iter().enumerate() {
        let apostrophe = matches!(character, '\'' | '\u{2019}');
        let inside_word = apostrophe
            && !current.is_empty()
            && characters
                .get(index + 1)
                .is_some_and(|next| next.is_alphanumeric());
        if character.is_alphanumeric() || inside_word {
            current.push(if apostrophe { '\'' } else { character });
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn edit_counts<T: Eq>(expected: &[T], actual: &[T]) -> EditCounts {
    let width = actual.len() + 1;
    let mut costs = vec![0_usize; (expected.len() + 1) * width];
    for row in 0..=expected.len() {
        costs[row * width] = row;
    }
    for (column, cost) in costs.iter_mut().take(width).enumerate() {
        *cost = column;
    }
    for row in 1..=expected.len() {
        for column in 1..=actual.len() {
            let substitution = costs[(row - 1) * width + column - 1]
                + usize::from(expected[row - 1] != actual[column - 1]);
            let deletion = costs[(row - 1) * width + column] + 1;
            let insertion = costs[row * width + column - 1] + 1;
            costs[row * width + column] = substitution.min(deletion).min(insertion);
        }
    }

    let mut counts = EditCounts::default();
    let (mut row, mut column) = (expected.len(), actual.len());
    while row > 0 || column > 0 {
        if row > 0
            && column > 0
            && expected[row - 1] == actual[column - 1]
            && costs[row * width + column] == costs[(row - 1) * width + column - 1]
        {
            row -= 1;
            column -= 1;
        } else if row > 0
            && column > 0
            && costs[row * width + column] == costs[(row - 1) * width + column - 1] + 1
        {
            counts.substitutions += 1;
            row -= 1;
            column -= 1;
        } else if row > 0 && costs[row * width + column] == costs[(row - 1) * width + column] + 1 {
            counts.deletions += 1;
            row -= 1;
        } else {
            counts.insertions += 1;
            column -= 1;
        }
    }
    counts
}

fn output_token_matches<T: Eq>(expected: &[T], actual: &[T]) -> Vec<bool> {
    let width = actual.len() + 1;
    let mut costs = vec![0_usize; (expected.len() + 1) * width];
    for row in 0..=expected.len() {
        costs[row * width] = row;
    }
    for (column, cost) in costs.iter_mut().take(width).enumerate() {
        *cost = column;
    }
    for row in 1..=expected.len() {
        for column in 1..=actual.len() {
            costs[row * width + column] = (costs[(row - 1) * width + column - 1]
                + usize::from(expected[row - 1] != actual[column - 1]))
            .min(costs[(row - 1) * width + column] + 1)
            .min(costs[row * width + column - 1] + 1);
        }
    }

    let mut matches = vec![false; actual.len()];
    let (mut row, mut column) = (expected.len(), actual.len());
    while row > 0 || column > 0 {
        if row > 0
            && column > 0
            && expected[row - 1] == actual[column - 1]
            && costs[row * width + column] == costs[(row - 1) * width + column - 1]
        {
            matches[column - 1] = true;
            row -= 1;
            column -= 1;
        } else if row > 0
            && column > 0
            && costs[row * width + column] == costs[(row - 1) * width + column - 1] + 1
        {
            row -= 1;
            column -= 1;
        } else if row > 0 && costs[row * width + column] == costs[(row - 1) * width + column] + 1 {
            row -= 1;
        } else {
            column -= 1;
        }
    }
    matches
}

fn contamination_rate(
    expected: &[String],
    actual: &[String],
    alternatives: &[Vec<String>],
) -> Option<f64> {
    let expected_matches = output_token_matches(expected, actual);
    let wrong_indices: Vec<usize> = expected_matches
        .iter()
        .enumerate()
        .filter_map(|(index, matched)| (!matched).then_some(index))
        .collect();
    if wrong_indices.is_empty() || alternatives.is_empty() {
        return None;
    }

    let alternative_matches: Vec<Vec<bool>> = alternatives
        .iter()
        .map(|alternative| output_token_matches(alternative, actual))
        .collect();
    let explained = wrong_indices
        .iter()
        .filter(|&&index| alternative_matches.iter().any(|matches| matches[index]))
        .count();
    Some(ratio(explained, wrong_indices.len()))
}

fn contains_contiguous(haystack: &[String], needle: &[String]) -> bool {
    !needle.is_empty()
        && haystack.len() > needle.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn looks_like_refusal(output: &str) -> bool {
    let lower = output.to_lowercase();
    [
        "can't provide",
        "cannot provide",
        "unable to provide",
        "can't quote",
        "cannot quote",
        "copyright",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}

fn rate(errors: usize, reference_length: usize) -> f64 {
    if reference_length == 0 {
        f64::from(errors > 0)
    } else {
        ratio(errors, reference_length)
    }
}

#[allow(clippy::cast_precision_loss)]
fn ratio(numerator: usize, denominator: usize) -> f64 {
    numerator as f64 / denominator as f64
}

const fn classification_name(classification: Classification) -> &'static str {
    match classification {
        Classification::ExactRequested => "exact_requested",
        Classification::TranslationConfusion => "translation_confusion",
        Classification::ExtraneousText => "extraneous_text",
        Classification::Refusal => "refusal",
        Classification::Empty => "empty",
        Classification::ProviderError => "provider_error",
        Classification::Partial => "partial",
    }
}

#[cfg(test)]
mod tests {
    use crate::{BibleReference, CaseStratum, PromptVariant};

    use super::*;

    fn case() -> BenchmarkCase {
        BenchmarkCase {
            case_id: "BQ-1".to_owned(),
            translation: "requested".to_owned(),
            reference: BibleReference {
                book: "Test".to_owned(),
                chapter: 1,
                verse_start: 1,
                verse_end: None,
            },
            stratum: CaseStratum::TranslationSensitive,
            prompt_variant: PromptVariant::Canonical,
        }
    }

    fn response(output: &str) -> ResponseRecord {
        ResponseRecord {
            case_id: "BQ-1".to_owned(),
            run_id: "run-1".to_owned(),
            provider: "fixture".to_owned(),
            model: "fixture".to_owned(),
            resolved_model: None,
            output: output.to_owned(),
            error: None,
            temperature: Some(0.0),
            reasoning_effort: None,
            seed: None,
            provider_request_id: None,
            system_fingerprint: None,
        }
    }

    fn reference(translation: &str, text: &str) -> ReferenceRecord {
        ReferenceRecord {
            translation: translation.to_owned(),
            reference: case().reference,
            text: text.to_owned(),
        }
    }

    #[test]
    fn exact_text_only_normalizes_technical_artifacts() {
        let score = score_response(
            &case(),
            &response("Jesus said, \"I am.\"\r\n"),
            &reference("requested", "Jesus said, “I am.”\n"),
            &[],
        );
        assert!(!score.exact_text);
        assert!(score.exact_words);
        assert!(score.word_error_rate.abs() < f64::EPSILON);
    }

    #[test]
    fn exact_text_normalizes_line_endings_and_unicode_nfc() {
        let score = score_response(
            &case(),
            &response("Cafe\u{301}\r\nline two"),
            &reference("requested", "Café\nline two"),
            &[],
        );
        assert!(score.exact_text);
    }

    #[test]
    fn exact_words_equates_straight_and_curly_apostrophes() {
        let score = score_response(
            &case(),
            &response("don't stop"),
            &reference("requested", "don’t stop"),
            &[],
        );
        assert!(!score.exact_text);
        assert!(score.exact_words);
        assert!(!score.extraneous_text);
    }

    #[test]
    fn edit_counts_distinguish_operations() {
        let score = score_response(
            &case(),
            &response("A changed C extra"),
            &reference("requested", "A B C"),
            &[],
        );
        assert_eq!(score.substitutions, 1);
        assert_eq!(score.insertions, 1);
        assert_eq!(score.deletions, 0);
    }

    #[test]
    fn exact_alternative_is_translation_confusion() {
        let score = score_response(
            &case(),
            &response("A X C"),
            &reference("requested", "A B C"),
            &[&reference("alternative", "A X C")],
        );
        assert_eq!(score.classification, Classification::TranslationConfusion);
        assert_eq!(
            score.exact_other_translation.as_deref(),
            Some("alternative")
        );
        assert_eq!(score.translation_contamination_rate, Some(1.0));
    }

    #[test]
    fn commentary_around_exact_target_is_extraneous() {
        let score = score_response(
            &case(),
            &response("Here it is: A B C"),
            &reference("requested", "A B C"),
            &[],
        );
        assert_eq!(score.classification, Classification::ExtraneousText);
        assert!(score.extraneous_text);
    }

    #[test]
    fn aggregate_is_defined_for_empty_input() {
        assert_eq!(aggregate_scores(&[]).responses, 0);
    }

    #[test]
    fn alternative_ties_are_explicit_and_order_independent() {
        let a = reference("a", "A X C");
        let b = reference("b", "A X C");
        let requested = reference("requested", "A B C");
        let exact = score_response(&case(), &response("A X C"), &requested, &[&b, &a]);
        let reordered = score_response(&case(), &response("A X C"), &requested, &[&a, &b]);
        assert_eq!(exact, reordered);
        assert_eq!(exact.exact_other_translations, vec!["a", "b"]);
        let report = crate::report::build_report(&[exact]);
        assert_eq!(
            report.requested_to_resembles["requested"]["_ambiguous_exact"],
            1
        );
        assert_eq!(report.exact_alternative_matches["requested"]["a"], 1);
        assert_eq!(report.exact_alternative_matches["requested"]["b"], 1);
        let approximate = score_response(&case(), &response("A X extra C"), &requested, &[&a, &b]);
        assert!(approximate.exact_other_translations.is_empty());
        assert_eq!(approximate.closest_translations, vec!["a", "b"]);
        assert_eq!(
            crate::report::build_report(&[approximate]).requested_to_resembles["requested"]["_ambiguous_closest"],
            1
        );
    }

    #[test]
    fn serialized_overlap_has_new_name_and_reads_legacy_alias() {
        let score = score_response(
            &case(),
            &response("A X C"),
            &reference("requested", "A B C"),
            &[&reference("other", "A X C")],
        );
        let json = serde_json::to_string(&score).unwrap();
        assert!(json.contains("alternative_edition_token_overlap"));
        assert!(!json.contains("translation_contamination_rate"));
        let legacy = json.replace(
            "alternative_edition_token_overlap",
            "translation_contamination_rate",
        );
        assert_eq!(serde_json::from_str::<ScoreRecord>(&legacy).unwrap(), score);
    }
}
