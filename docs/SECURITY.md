# Publication safety

The repository has two complementary publication guards:

- `.githooks/pre-commit` examines the exact staged snapshot before each commit.
- `.github/workflows/security.yml` examines every tracked file on pushes and pull requests.

Both guards block `data/hidden`, the hidden sampling secret, environment and
credential files, private-key containers, and high-confidence provider or cloud
credential formats. Findings report only the affected path and credential kind;
secret values are never echoed.

Enable the versioned hook in a clone with:

```console
git config core.hooksPath .githooks
```

The checks can also be run directly:

```console
cargo run -- guard-staged
cargo run -- guard-tracked
```

The guard is defense in depth, not a replacement for provider-side secret
scanning or credential rotation. If a secret reaches Git history, revoke it
immediately before removing it from history.
