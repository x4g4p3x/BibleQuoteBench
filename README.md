# BibleQuoteBench

BibleQuoteBench measures whether a model can reproduce the exact text of a
specified Bible translation and edition from internal recall. Its primary track
is closed-book: no retrieval, browsing, Bible API, tools, or RAG.

The v0.2 implementation includes:

- edition-pinned translation metadata;
- strict JSON/JSONL schemas through Rust types;
- dataset integrity and licensing-invariant validation;
- deterministic canonical prompt rendering;
- `ExactText`, `ExactWords`, WER, CER, and edit-operation counts;
- exact-other-translation detection and alternative-edition token overlap;
- refusal and extraneous-output classification;
- aggregate summaries and an auditable JSONL result format;
- provenance-preserving USFM zip/directory imports with SHA-256 locks;
- a deterministic shared-reference sampler and 20/80 public/hidden split;
- live OpenAI, Anthropic, Gemini, xAI, and OpenAI-compatible adapters;
- budgeted execution, durable progress, resume, token accounting, and cutoff detection;
- per-model, translation, and stratum reporting, a requested-to-resembles
  matrix, and repeated-run stability statistics;
- complete-run manifests, configuration checks, and paired cluster-bootstrap intervals;
- separate copy controls, fixed prompt variants, and three-verse passage diagnostics;
- offline interactive HTML reports with uncertainty charts and a failure explorer;
- a reproducible [synthetic pilot with annotated failures](docs/pilot/v0.2/README.md).

This measures edition-specific quotation reliability under closed-book prompting.
It does not establish theological understanding, general intelligence, or whether
a passage appeared in training. Hidden selection protects against tuning to chosen
questions; the underlying Bible text may already be familiar to a model.

See the [v0.2 release notes](docs/RELEASE-v0.2.md) for changes and limitations.

## Quick start

```console
cargo run -- validate
cargo run -- score --responses data/dev/responses.example.jsonl --output scores.jsonl
cargo run -- summarize --scores scores.jsonl
cargo run -- report --scores scores.jsonl
```

The example report is descriptive. For full model comparisons, use `analyze`
with response files and their companion manifests; see [scoring](docs/SCORING.md).
No live GPT-6 pilot has been run. The saved EUR 20 pilot plan is held at the user's
request, with zero spend.

Open the [interactive synthetic pilot](docs/pilot/v0.2/index.html) by downloading
the HTML file and opening it in your browser. Each `analyze` run also writes
`analysis.html`. Combine existing analyses into one portable report with:

```console
cargo run -- visualize --analysis docs/pilot/v0.2/canonical/analysis.json --analysis docs/pilot/v0.2/copy_control/analysis.json --output results/report.html
```

The report works without a server or an internet connection. See
[result visualization](docs/VISUALIZATION.md) for controls and interpretation.

Before a paid run, read [budgeted execution](docs/EXECUTION.md). The prepared
Fable validation preview is offline and keeps spending disabled:

```powershell
./scripts/preview-fable-pilot.ps1
```

Run the quality gates with:

```console
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo run -- guard-tracked
```

Before the first commit in a clone, enable the repository's publication guard:

```console
git config core.hooksPath .githooks
```

It blocks hidden evaluation material, private seed files, environment files,
private keys, and recognizable provider credentials from being committed. CI
applies the same policy to every tracked file. See
[publication safety](docs/SECURITY.md).

## Data boundary

Cases contain references, translation identifiers, strata, and prompt variants.
Reference text lives in a separate corpus. That separation allows a future hosted
evaluator to keep licensed text private while publishing cases and numeric scores.
The validator rejects licensed-private translations that claim their reference
text may be redistributed.

The checked-in development dataset contains 100 shared references × three
public-domain translations, or 300 cases. The generated hidden set contains 400
shared references × three translations, or 1,200 cases. Hidden files are ignored;
their pre-evaluation SHA-256 commitment is public in the release manifest. Their
selection seed is generated locally, ignored by Git, and must be backed up by the
evaluator.

Reproduce the public split—and the hidden split when the retained private seed
is present—from locked official USFM sources on Windows with:

```powershell
./scripts/build-dataset.ps1
```

See [dataset construction](docs/DATASET.md), [provider execution](docs/PROVIDERS.md),
the normative [scoring contract](docs/SCORING.md), and the system
[architecture](docs/ARCHITECTURE.md).

## Development

The repository enforces formatting, compilation, strict Clippy, cross-platform
tests, warning-free Rust documentation, an 80% line-coverage floor, and the
publication guard in CI. Run the same suite locally on Windows with:

```powershell
./scripts/check.ps1 -Coverage
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the testing strategy and change
checklist.

## Provenance

- Berean Standard Bible: 2025 third printing, from the official Berean Bible
  downloads. The Berean Bible was dedicated to the public domain in 2023.
- American Standard Version: 1901 standard edition, via eBible.org, public domain.
- World English Bible Classic: 2020 stable text edition, via eBible.org, public
  domain. `World English Bible` is also a trademark; the project preserves the
  official text and uses the name only to identify it.

Source and license URLs are machine-readable in
`data/dev/translations.json`. Bible text data is public domain; the benchmark
software is MIT-licensed.

## Scope boundary

The primary v0.2 score remains single-verse closed-book recall. Copy controls,
prompt variants, and three-verse passages are separate diagnostic tracks.
Publisher-authorized private editions, hosted hidden-set evaluation, and a public
leaderboard remain future work. The v0.1 public dataset is preserved under
`data/releases/v0.1/dev`; do not compare scores across versions as if the cases
were unchanged.
