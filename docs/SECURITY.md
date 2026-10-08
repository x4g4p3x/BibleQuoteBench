# Publication safety

The repository has two complementary publication guards:

- `.githooks/pre-commit` examines the exact staged snapshot before each commit.
- `.github/workflows/security.yml` examines every tracked file on pushes and pull requests.

Both guards block `data/hidden`, the hidden sampling secret, environment and
credential files, private-key containers, and high-confidence provider or cloud
credential formats. Findings report only the affected path and credential kind;
secret values are never echoed.

Recognizable hidden case identifiers are also blocked inside renamed JSON or
report artifacts, including the expected-case lists in run manifests. Keep hidden
responses and manifests in ignored evaluator storage. Annotated examples are
suppressed for hidden case IDs and private licensed catalogs. These controls cannot
identify every possible excerpt of public-domain scripture; they are not a secrecy
guarantee for arbitrary renamed reference files.

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

## Restricted recall boundary

The [restricted subscription runner](RESTRICTED.md) replaces native client request
context and offers no tools. It validates the complete response before returning
any bytes that could trigger a client tool. MCP callers cannot submit answers or
inject context. Attempts are durable before execution; uncertain attempts cannot
be replayed, and accepted responses are validated against their audits on resume.

The trusted local executable supplies subscription authentication from a private
temporary credential copy. API-key authentication is rejected, and only the fixed
Codex subscription endpoint is used. Headers and tokens are excluded from audits.
Native credential refreshes preserve the original file permissions and reject an
observed source/account change. Run on a trusted user account, avoid concurrent
sign-in changes, and keep audit responses in ignored evaluator storage.

This boundary does not control other tools in the desktop host, make local
artifacts tamper-proof, or verify service internals. The distinct evidence label
and report limitations describe those remaining trust assumptions.
