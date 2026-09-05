# Architecture

BibleQuoteBench is a deterministic pipeline with explicit trust boundaries:

```text
official USFM archives
        │ import + SHA-256 locks
        ▼
translation corpora
        │ deterministic stratified sampling
        ├──────────────► public development cases and references
        │
        └──────────────► private hidden cases and references
                              │
cases ──► closed-book prompts ──► provider responses
                                      │
reference text ───────────────────────┤ scoring
                                      ▼
                          score JSONL ──► reports
```

## Components

- `domain` defines the versioned JSON and JSONL records exchanged by each stage.
- `importer` converts official USFM archives or directories into canonical
  single-verse records and provenance locks.
- `sampling` creates non-overlapping public and hidden strata. Public selection
  uses the committed seed; hidden selection uses an ignored private seed.
- `prompt` renders fixed, edition-pinned instructions without reference text.
- `provider` executes stateless closed-book requests and retains provider
  metadata and per-case failures.
- `execution` locks a shared campaign budget, reserves each request, and saves
  resumable checkpoints with token and completion accounting.
- `scoring` implements the normative exact and edit-distance metrics.
- `report` produces grouped summaries, translation resemblance, and stability.
- `study` binds complete runs to manifests and rejects incomparable observations.
- `statistics` bootstraps stratified reference clusters and paired differences.
- `visualization` embeds analyses into an offline HTML viewer with inline assets;
  it displays precomputed statistics and keeps each track separate.
- `pilot` prepares separate diagnostic tracks and synthetic release evidence.
- `validation` enforces cross-file, licensing, and markup-cleanliness invariants.
- `security` prevents private evaluation material and credentials from entering
  the Git index.

The CLI in `main.rs` composes these modules without weakening their validation.

## Determinism and provenance

Every source archive, contained USFM artifact, imported corpus, and released case
set is SHA-256-addressed. Stable ordering precedes serialization and hashing.
Sampler tie-breaking uses domain-separated hashes instead of iteration order.
Repository text files are pinned to LF line endings so JSONL commitments do not
change across Windows and Linux checkouts.

The public seed reproduces the development set. A retained private seed
reproduces the hidden set; only the hidden case-file commitment is published.
Changing either source bytes or importer output causes the build to fail against
the committed locks.

## Data and security boundaries

Cases identify a reference and edition but do not contain expected text.
Reference corpora are separate so licensed editions can remain evaluator-only.
Recall requests contain the rendered prompt without reference text and offer no
tools, retrieval, browsing, or conversation history. The separate copy-control
track explicitly supplies reference text in a marked prompt section. The
execution and analysis paths reject mixed tracks; loopback tests verify this
boundary in actual request bodies.

The pre-commit and CI publication guards inspect Git-indexed bytes rather than
only working-tree filenames. They are defense in depth; evaluator hosts must also
use ordinary secret management, access control, and provider credential rotation.

## Failure model

Schema, provenance, and sampling failures stop the operation. Provider failures
are retained per case by default so missing observations remain visible; callers
may request fail-fast behavior. Reports operate only on explicit score records
and never infer absent model responses.

The validated `analyze` path additionally requires exactly one response per intended
case per run, including explicit provider failures. It binds response, dataset,
catalog, and prompt digests and isolates model versions and configurations before
inference. Plain `report` remains exploratory and advertises its narrower guarantees.
