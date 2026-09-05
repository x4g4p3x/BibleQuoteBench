# BibleQuoteBench v0.2

This release strengthens hidden selection and model-comparison validity, and adds
diagnostics that help explain failures while preserving the strict main score.

## Changes

- Hidden famous references are sampled from a larger public candidate pool;
  extreme-length and translation-sensitive references are privately sampled
  within ranked candidate pools. The famous quota changes from 50 to 20, with
  those 30 slots transferred to random references. Total case counts stay fixed.
- Complete-run manifests bind cases, corpus, catalog, prompts, model configuration,
  and responses. Validated analysis rejects incomplete, duplicated, altered, or
  unbalanced observations and isolates versions and settings.
- Reference-cluster confidence intervals and paired differences keep editions
  and repeated runs together. Provider errors have separate operational and
  conditional-recall accounting. Failed requests cannot inflate stability.
- Exact other-edition matches, approximate resemblance, and ambiguous matches
  are distinct. Alternative-edition token overlap replaces the causal-sounding
  contamination label. Annotated examples expose the actual wording changes.
- Copy controls, fixed prompt variants, and three-verse passages have separate
  datasets and reports. Punctuation-only differences retain `ExactWords` success
  without being mislabeled as extraneous prose.

## Reproduce the release

```powershell
./scripts/build-release.ps1
./scripts/check.ps1 -Coverage
```

The release script rebuilds the dataset twice and verifies identical artifacts
with the retained hidden seed, then prepares the public pilot and regenerates
synthetic evidence twice to check determinism. It makes no model API calls.
It may download the locked official source archives if they are missing.

Individual steps are also available:

```console
cargo run -- prepare-pilot --corpus data/corpus/bsb.jsonl --corpus data/corpus/asv.jsonl --corpus data/corpus/web.jsonl
cargo run -- synthetic-pilot
```

The [pilot report](pilot/v0.2/README.md) covers five tracks, two synthetic response
generators, and three repetitions: 1,080 constructed observations across 30 runs.
Each track has 12 reference clusters and 36 cases per run. This exercises the
real scoring and inference pipeline, including annotated failures, without
claiming any live model performance. These artifacts are ready for publication
with the repository; this change does not push, tag, or deploy a hosted release.

## Live pilot status

The requested GPT-6 Astra pilot with `max` reasoning is held at the user's request. The saved
ceiling is EUR 20, and model API spend is EUR 0. The API model name and selected `max` reasoning level are documented; see
[provider notes](PROVIDERS.md). Live execution and any results
publication from it remain pending the user's later instruction.

## Interpretation and compatibility

The primary result measures edition-specific quotation reliability under
closed-book prompting. It does not establish theological understanding, general
intelligence, or the provenance of memorized wording. A hidden question list is
not unseen scripture. Candidate pools and underlying texts remain public.

The current report describes a deliberate benchmark mixture, not the frequency
of successful quotation across the whole Bible. The pilot is smaller and uses a
different diagnostic mixture. Bootstrap intervals are conditional on observed
runs; small strata and multiple comparisons require care. No automatic winner
claim is made. See the [normative scoring contract](SCORING.md).

The v0.1 public dataset is retained in `data/releases/v0.1/dev`, alongside its
original sampling configuration and release manifest at their original paths.
Its scores must not be pooled with v0.2. Legacy response records and the former
overlap-field name remain readable for descriptive scoring. Validated comparison
requires v0.2 manifests and the exact matching dataset.

## Local validation

The Windows quality suite passed 52 tests, strict Clippy, warning-free Rust
documentation, the tracked-file publication guard, and 91.28% line coverage
(80% required). An additional scan included new, untracked release artifacts and
found no recognizable credentials or hidden case identifiers. The archived v0.1
public cases match their original SHA-256 commitment. These are local results;
cross-platform hosted CI has not been run for this uncommitted change.

Two final release builds produced byte-identical public and hidden dataset
artifacts. Two pilot builds likewise produced byte-identical diagnostic datasets,
synthetic responses, manifests, JSON analyses, Markdown reports, and examples.
