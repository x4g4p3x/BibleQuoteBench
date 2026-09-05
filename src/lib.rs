//! Trusted domain, import, sampling, execution, scoring, and reporting core for
//! `BibleQuoteBench`.
//!
//! The crate keeps benchmark cases separate from reference text so that the same
//! engine can support both redistributable development data and private licensed
//! evaluation corpora. See the repository's architecture and scoring documents
//! for the reproducibility and trust-boundary contracts.

pub mod domain;
pub mod importer;
pub mod io;
pub mod pilot;
pub mod prompt;
pub mod provider;
pub mod report;
pub mod sampling;
pub mod scoring;
pub mod security;
pub mod statistics;
pub mod study;
pub mod validation;

pub use domain::{
    BenchmarkCase, BibleReference, CaseStratum, Classification, CorpusLock, LicenseKind,
    PromptVariant, ReferenceRecord, ResponseRecord, ScoreRecord, SourceArtifact,
    TranslationCatalog, TranslationSpec,
};
pub use prompt::render_prompt;
pub use scoring::{ScoreSummary, aggregate_scores, score_response};
pub use validation::validate_dataset;
