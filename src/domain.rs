//! Versioned serializable records shared across the benchmark pipeline.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BibleReference {
    pub book: String,
    pub chapter: u16,
    pub verse_start: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verse_end: Option<u16>,
}

impl BibleReference {
    pub fn end_verse(&self) -> u16 {
        self.verse_end.unwrap_or(self.verse_start)
    }

    /// Checks the structural invariants of a Scripture reference.
    ///
    /// # Errors
    ///
    /// Returns an explanation when the book is empty or a chapter/verse range is invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.book.trim().is_empty() {
            return Err("book must not be empty".to_owned());
        }
        if self.chapter == 0 {
            return Err("chapter must be greater than zero".to_owned());
        }
        if self.verse_start == 0 {
            return Err("verse_start must be greater than zero".to_owned());
        }
        if self.end_verse() < self.verse_start {
            return Err("verse_end must be greater than or equal to verse_start".to_owned());
        }
        Ok(())
    }
}

impl fmt::Display for BibleReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.end_verse() == self.verse_start {
            write!(
                formatter,
                "{} {}:{}",
                self.book, self.chapter, self.verse_start
            )
        } else {
            write!(
                formatter,
                "{} {}:{}-{}",
                self.book,
                self.chapter,
                self.verse_start,
                self.end_verse()
            )
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStratum {
    ExtremelyFamous,
    WellKnown,
    Moderate,
    Random,
    Obscure,
    VeryObscure,
    TranslationSensitive,
    SimilarTranslations,
    ShortVerse,
    LongVerse,
    Passage,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptVariant {
    #[default]
    Canonical,
    Concise,
    WordForWord,
    /// Separate open-book diagnostic; never part of closed-book recall.
    CopyControl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkCase {
    pub case_id: String,
    pub translation: String,
    pub reference: BibleReference,
    pub stratum: CaseStratum,
    #[serde(default)]
    pub prompt_variant: PromptVariant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseKind {
    PublicDomain,
    LicensedPrivate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationSpec {
    pub id: String,
    pub name: String,
    pub abbreviation: String,
    pub edition: String,
    pub license_kind: LicenseKind,
    pub license_url: String,
    pub source_url: String,
    pub redistribute_reference_text: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationCatalog {
    pub schema_version: u16,
    pub translations: Vec<TranslationSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceRecord {
    pub translation: String,
    pub reference: BibleReference,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceArtifact {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusLock {
    pub schema_version: u16,
    pub translation: String,
    pub edition: String,
    pub source_url: String,
    pub source_sha256: String,
    pub importer_version: String,
    pub artifacts: Vec<SourceArtifact>,
    pub reference_count: usize,
    pub corpus_sha256: String,
}

/// Provider-reported billable usage. Output includes thinking tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionMetadata {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub stop_reason: Option<String>,
    pub truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accounted_nanoeur: Option<u64>,
    #[serde(default)]
    pub reservation_retained: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseRecord {
    pub case_id: String,
    pub run_id: String,
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<String>,
    pub output: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    ExactRequested,
    TranslationConfusion,
    ExtraneousText,
    Refusal,
    Empty,
    ProviderError,
    Truncated,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub struct ScoreRecord {
    pub case_id: String,
    pub run_id: String,
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<String>,
    pub response_sha256: String,
    pub requested_translation: String,
    pub reference: BibleReference,
    pub stratum: CaseStratum,
    pub exact_text: bool,
    pub exact_words: bool,
    pub word_error_rate: f64,
    pub character_error_rate: f64,
    pub word_accuracy: f64,
    pub insertions: usize,
    pub deletions: usize,
    pub substitutions: usize,
    pub refusal: bool,
    pub extraneous_text: bool,
    pub classification: Classification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_other_translation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exact_other_translations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closest_translation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closest_translations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(
        rename = "alternative_edition_token_overlap",
        alias = "translation_contamination_rate"
    )]
    pub translation_contamination_rate: Option<f64>,
}
