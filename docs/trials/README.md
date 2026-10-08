# Observed interactive trials

The [300-case accuracy comparison](2026-10-09-300-case-comparison/README.md)
shows the completed Grok 4.7 xHigh and Claude Opus 5.5 Max Cursor runs alongside
the restricted GPT-6.1 Sol Max Codex run. It also appears at the top of the
repository README.

This archive contains actual model answers obtained through desktop or CLI
subscription clients. Interactive Cursor records retain the `interactive_mcp`
evidence label; the restricted Codex comparison has stronger request-gate
evidence. These are descriptive results with client and conversation limitations.
They are separate from the [constructed v0.2 pilot](../pilot/v0.2/README.md),
controlled provider comparisons, and the [restricted Codex runner](../RESTRICTED.md).

For subscription runs, disclose the answering client and tool restrictions with
every result. See [client paths and token accounting](../TOKEN-USAGE.md) for
usage coverage, account-export reconciliation and comparison limits. Full
300-case answer archives and their aggregate token reports are kept in the
private evaluator repository.

| Date | Requested model and client | Edition | Cases | ExactText / ExactWords | Mean word accuracy | Report |
| --- | --- | --- | ---: | ---: | ---: | --- |
| 2026-10-08 | Grok 4.7 xHigh, Fast off; Cursor CLI | BSB 2025 third printing | 10 | 20% / 20% | 79.49% | [Methodology and results](2026-10-08-grok47-xhigh-cursor/README.md) |

The first trial includes four downloadable figures, an
[offline interactive case explorer](2026-10-08-grok47-xhigh-cursor/index.html),
the actual submitted answers, a public reference subset, score records, and
sanitized provenance. Download the HTML and open it locally to use the explorer;
GitHub displays HTML source rather than executing it.

No model ranking or pooled estimate is implied by this table. Case selection,
edition, settings, evidence strength, and repetitions must be considered before
comparing future runs. The paid GPT-6 pilot remains held with zero spend.
