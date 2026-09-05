//! Aggregate report construction and deterministic Markdown rendering.

use std::{
    collections::{BTreeMap, HashMap},
    fmt::Write as _,
};

use serde::{Deserialize, Serialize};

use crate::{CaseStratum, Classification, ScoreRecord, ScoreSummary, aggregate_scores};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StabilitySummary {
    pub repeated_cases: usize,
    pub mean_output_consistency: f64,
    pub mean_exact_recall: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub overall: ScoreSummary,
    pub by_model: BTreeMap<String, ScoreSummary>,
    pub by_translation: BTreeMap<String, ScoreSummary>,
    pub by_stratum: BTreeMap<String, ScoreSummary>,
    pub requested_to_resembles: BTreeMap<String, BTreeMap<String, usize>>,
    pub exact_alternative_matches: BTreeMap<String, BTreeMap<String, usize>>,
    pub stability: BTreeMap<String, StabilitySummary>,
}

/// Builds all aggregate, confusion, and repeated-run stability views.
pub fn build_report(scores: &[ScoreRecord]) -> BenchmarkReport {
    BenchmarkReport {
        overall: aggregate_scores(scores),
        by_model: grouped_summary(scores, model_key),
        by_translation: grouped_summary(scores, |score| score.requested_translation.clone()),
        by_stratum: grouped_summary(scores, |score| stratum_name(score.stratum).to_owned()),
        requested_to_resembles: confusion_matrix(scores),
        exact_alternative_matches: exact_matches(scores),
        stability: stability(scores),
    }
}

/// Renders a benchmark report as a stable Markdown document.
pub fn render_markdown(report: &BenchmarkReport) -> String {
    let mut output = String::from(
        "# BibleQuoteBench report\n\nExploratory descriptive report: supplied rows only; coverage and configuration comparability are not verified here. Use `analyze` for validated comparisons.\n\n\
         ## Overall\n\n\
         | Responses | ExactText | ExactWords | Word accuracy | Refusals | Provider errors | Translation confusion |\n\
         | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    output.push_str(&summary_row(&report.overall));
    output.push_str("\n## Models\n\n| Provider / model | Responses | ExactText | ExactWords | Word accuracy | Confusion |\n| --- | ---: | ---: | ---: | ---: | ---: |\n");
    for (name, summary) in &report.by_model {
        let _ = writeln!(
            output,
            "| {} | {} | {} | {} | {} | {} |",
            escape_table(name),
            summary.responses,
            percent(summary.exact_text_rate),
            percent(summary.exact_words_rate),
            percent(summary.mean_word_accuracy),
            percent(summary.translation_confusion_rate)
        );
    }
    output.push_str("\n## Translations\n\n| Translation | Responses | ExactText | ExactWords | Word accuracy | Confusion |\n| --- | ---: | ---: | ---: | ---: | ---: |\n");
    append_group_table(&mut output, &report.by_translation);
    output.push_str("\n## Strata\n\n| Stratum | Responses | ExactText | ExactWords | Word accuracy | Confusion |\n| --- | ---: | ---: | ---: | ---: | ---: |\n");
    append_group_table(&mut output, &report.by_stratum);
    output.push_str("\n## Requested → resembles\n\nCounts use exact requested/other-edition matches first, then the closest alternative when it is strictly closer; remaining errors are `_unclassified`.\n\n");
    for (requested, produced) in &report.requested_to_resembles {
        let _ = write!(output, "- **{}**: ", escape_table(requested));
        output.push_str(
            &produced
                .iter()
                .map(|(name, count)| format!("{}={count}", escape_table(name)))
                .collect::<Vec<_>>()
                .join(", "),
        );
        output.push('\n');
    }
    output.push_str("\nExact other-edition matches (separate from approximate resemblance):\n\n");
    for (requested, matches) in &report.exact_alternative_matches {
        let _ = writeln!(
            output,
            "- {}: {}",
            escape_table(requested),
            matches
                .iter()
                .map(|(name, count)| format!("{}={count}", escape_table(name)))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    output.push_str("\n## Stability\n\n| Provider / model | Repeated cases | Output consistency | Exact recall |\n| --- | ---: | ---: | ---: |\n");
    for (name, summary) in &report.stability {
        let _ = writeln!(
            output,
            "| {} | {} | {} | {} |",
            escape_table(name),
            summary.repeated_cases,
            percent(summary.mean_output_consistency),
            percent(summary.mean_exact_recall)
        );
    }
    output
}

fn grouped_summary(
    scores: &[ScoreRecord],
    key: impl Fn(&ScoreRecord) -> String,
) -> BTreeMap<String, ScoreSummary> {
    let mut groups: BTreeMap<String, Vec<ScoreRecord>> = BTreeMap::new();
    for score in scores {
        groups.entry(key(score)).or_default().push(score.clone());
    }
    groups
        .into_iter()
        .map(|(name, scores)| (name, aggregate_scores(&scores)))
        .collect()
}

fn confusion_matrix(scores: &[ScoreRecord]) -> BTreeMap<String, BTreeMap<String, usize>> {
    let mut matrix: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for score in scores {
        let resembles = if score.classification == Classification::ProviderError {
            "_provider_error"
        } else if score.classification == Classification::Empty {
            "_empty"
        } else if score.classification == Classification::Refusal {
            "_refusal"
        } else if score.classification == Classification::ExactRequested {
            score.requested_translation.as_str()
        } else if score.exact_other_translations.len() > 1 {
            "_ambiguous_exact"
        } else if let Some(translation) = score.exact_other_translation.as_deref() {
            translation
        } else if score.closest_translations.len() > 1 {
            "_ambiguous_closest"
        } else {
            score
                .closest_translation
                .as_deref()
                .unwrap_or("_unclassified")
        };
        *matrix
            .entry(score.requested_translation.clone())
            .or_default()
            .entry(resembles.to_owned())
            .or_insert(0) += 1;
    }
    matrix
}

fn exact_matches(scores: &[ScoreRecord]) -> BTreeMap<String, BTreeMap<String, usize>> {
    let mut matrix: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for score in scores
        .iter()
        .filter(|score| score.classification == Classification::TranslationConfusion)
    {
        let matches: Vec<_> = if score.exact_other_translations.is_empty() {
            score.exact_other_translation.iter().collect()
        } else {
            score.exact_other_translations.iter().collect()
        };
        for translation in matches {
            *matrix
                .entry(score.requested_translation.clone())
                .or_default()
                .entry(translation.clone())
                .or_default() += 1;
        }
    }
    matrix
}

#[allow(clippy::cast_precision_loss)]
fn stability(scores: &[ScoreRecord]) -> BTreeMap<String, StabilitySummary> {
    let mut cases: BTreeMap<(String, String), Vec<&ScoreRecord>> = BTreeMap::new();
    for score in scores {
        if score.classification == Classification::ProviderError {
            continue;
        }
        cases
            .entry((model_key(score), score.case_id.clone()))
            .or_default()
            .push(score);
    }
    let mut per_model: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();
    for ((model, _), mut runs) in cases {
        runs.sort_by_key(|score| &score.run_id);
        runs.dedup_by_key(|score| &score.run_id);
        if runs.len() < 2 {
            continue;
        }
        let mut hashes: HashMap<&str, usize> = HashMap::new();
        for score in &runs {
            *hashes.entry(score.response_sha256.as_str()).or_insert(0) += 1;
        }
        let consistency = ratio(
            *hashes.values().max().expect("runs are non-empty"),
            runs.len(),
        );
        let exact = ratio(
            runs.iter().filter(|score| score.exact_text).count(),
            runs.len(),
        );
        per_model
            .entry(model)
            .or_default()
            .push((consistency, exact));
    }
    per_model
        .into_iter()
        .map(|(model, values)| {
            let count = values.len();
            let consistency = values.iter().map(|value| value.0).sum::<f64>() / count as f64;
            let exact = values.iter().map(|value| value.1).sum::<f64>() / count as f64;
            (
                model,
                StabilitySummary {
                    repeated_cases: count,
                    mean_output_consistency: consistency,
                    mean_exact_recall: exact,
                },
            )
        })
        .collect()
}

fn append_group_table(output: &mut String, groups: &BTreeMap<String, ScoreSummary>) {
    for (name, summary) in groups {
        let _ = writeln!(
            output,
            "| {} | {} | {} | {} | {} | {} |",
            escape_table(name),
            summary.responses,
            percent(summary.exact_text_rate),
            percent(summary.exact_words_rate),
            percent(summary.mean_word_accuracy),
            percent(summary.translation_confusion_rate)
        );
    }
}

fn summary_row(summary: &ScoreSummary) -> String {
    format!(
        "| {} | {} | {} | {} | {} | {} | {} |\n",
        summary.responses,
        percent(summary.exact_text_rate),
        percent(summary.exact_words_rate),
        percent(summary.mean_word_accuracy),
        percent(summary.refusal_rate),
        percent(summary.provider_error_rate),
        percent(summary.translation_confusion_rate)
    )
}

fn model_key(score: &ScoreRecord) -> String {
    format!(
        "{} / {} / {}",
        score.provider,
        score.model,
        score.resolved_model.as_deref().unwrap_or("unresolved")
    )
}

const fn stratum_name(stratum: CaseStratum) -> &'static str {
    match stratum {
        CaseStratum::ExtremelyFamous => "extremely_famous",
        CaseStratum::WellKnown => "well_known",
        CaseStratum::Moderate => "moderate",
        CaseStratum::Random => "random",
        CaseStratum::Obscure => "obscure",
        CaseStratum::VeryObscure => "very_obscure",
        CaseStratum::TranslationSensitive => "translation_sensitive",
        CaseStratum::SimilarTranslations => "similar_translations",
        CaseStratum::ShortVerse => "short_verse",
        CaseStratum::LongVerse => "long_verse",
        CaseStratum::Passage => "passage",
    }
}

fn percent(value: f64) -> String {
    format!("{:.2}%", value * 100.0)
}

fn escape_table(value: &str) -> String {
    value.replace('|', "\\|")
}

#[allow(clippy::cast_precision_loss)]
fn ratio(numerator: usize, denominator: usize) -> f64 {
    numerator as f64 / denominator as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_report_renders() {
        let report = build_report(&[]);
        let markdown = render_markdown(&report);
        assert!(markdown.contains("# BibleQuoteBench report"));
        assert!(markdown.contains("| 0 | 0.00%"));
    }
}
