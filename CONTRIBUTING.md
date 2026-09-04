# Contributing to BibleQuoteBench

BibleQuoteBench treats reproducibility, exact textual fidelity, and hidden-set
confidentiality as correctness requirements. Changes should remain small enough
to review and must preserve those contracts.

## Development setup

Install Rust 1.85 or newer with the `rustfmt` and `clippy` components. For local
coverage, also install `llvm-tools-preview` and `cargo-llvm-cov` 0.9.0.

On Windows, run the complete local quality suite with:

```powershell
./scripts/check.ps1 -Coverage
```

The non-coverage suite does not require `cargo-llvm-cov`:

```powershell
./scripts/check.ps1
```

Enable the versioned pre-commit publication guard once per clone:

```console
git config core.hooksPath .githooks
```

## Required tests

Use the narrowest useful layer:

- Unit tests for normalization, parsing, deterministic selection, scoring, and
  failure behavior.
- Loopback HTTP contract tests for provider request and response formats. These
  must never make paid calls.
- CLI integration tests for complete user workflows and generated artifacts.
- Dataset reconstruction for changes to importing, sampling, locks, or schemas.

CI requires at least 80% line coverage across all targets and features. Coverage
is a regression floor, not permission to leave critical behavior untested.

## Change checklist

1. Add or update tests for observable behavior and relevant failure cases.
2. Update Rust API comments and repository documentation when contracts change.
3. Run `./scripts/check.ps1 -Coverage`.
4. For dataset changes, run `./scripts/build-dataset.ps1` twice and confirm the
   retained hidden seed produces the same commitment.
5. Never stage hidden cases, reference text, sampling secrets, `.env` files, or
   provider credentials.

Do not silently relax a score, coverage threshold, provenance digest, or
publication guard. Such changes require an explicit rationale in review.
