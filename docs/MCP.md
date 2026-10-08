# Interactive trials over MCP

For enforced tool-free answers through a separate Codex subscription model, use
the [restricted server](RESTRICTED.md). It works with all three desktop clients.
The workflow below describes the ordinary interactive server.

The local MCP server lets the assistant in Codex desktop, Claude Desktop, or
Cursor answer benchmark prompts and retain scored results without the benchmark
calling a model API. It needs no API key and makes no network requests. The
assistant still uses the client app's normal subscription or usage allowance.

These are **interactive trials**, labeled `interactive_mcp`, with self-reported
model identity. Shared conversation context, client instructions, unknown model sampling
settings, and access to other tools prevent treating these as controlled
closed-book provider runs. Start a fresh conversation and disable retrieval tools
where possible. Use the public development data for these trials; any case sent
to an assistant becomes part of that conversation and may leave the local machine.
The server does not expose reference text or use MCP sampling.

For an observed end-to-end example, see the
[8 October 2026 Grok 4.7 xHigh Cursor CLI trial](trials/2026-10-08-grok47-xhigh-cursor/README.md).
It publishes all ten answers, scoring inputs, charts, and sanitized provenance,
including client isolation, denied tool categories, observed call counts, and
the limits of those restrictions. It retains the `interactive_mcp` label.

## Connect desktop clients

Build the executable once:

```powershell
cargo build --locked --release
```

On PowerShell 7, configure any or all supported clients:

```powershell
./scripts/configure-mcp.ps1 -Client codex,claude,cursor
```

This registers `biblequotebench` in each client's personal settings. Codex uses
`codex mcp add`; Claude and Cursor receive a `mcpServers` entry with an absolute
executable and absolute dataset paths. Existing JSON settings are preserved and
backed up before editing. Default paths are `~/.cursor/mcp.json` and, on Windows,
`%APPDATA%/Claude/claude_desktop_config.json`. Existing Windows Store Claude profiles
are detected under `%LOCALAPPDATA%/Packages/Claude_*/LocalCache/Roaming/Claude`
and take precedence over the conventional path. Use Claude's Developer settings to
locate its configuration if your installation uses a different path, then pass
`-Client claude -ConfigPath <path>`. Restart the clients or reload MCP connections.
No running app is automatically restarted.

The default server starts idle, with ten BSB cases as trial defaults. No model or
run ID is stored in client configuration. Start a fresh chat and ask the assistant:

> Use the BibleQuoteBench MCP tools. Call begin_trial with run_id `trial-01-codex`
> and the explicit model label shown in the model picker. Answer each next_case
> prompt from internal recall only, without browsing, retrieval, Bible APIs, or
> inspecting local files. Send only the exact passage text as output to
> submit_answer, using the issued case_id. Do not revise answers. Repeat until
> complete, call finish_run, and report the summary.

Use a unique run ID for each model or repetition. After finishing, `begin_trial`
can start another trial through the same connection. To continue after a restart,
call `resume_trial` with the saved run ID; its original model label and selection
settings are restored. Once a prompt has been issued, finish the active trial
before switching runs. Existing runs are preserved; `begin_trial` never replaces
one. A repeated request with the same active trial settings is safe to retry.

Optional `begin_trial` arguments `case_limit`, `translation`, and `seed` override
the operator's defaults. Translation IDs include `asv-1901`, `web-classic-2020`,
and `bsb-2025-third-printing`. New trials use `stratified_reference_v1`: references
are ordered by a seed-derived hash within each difficulty stratum. The sample
includes every available stratum when enough reference groups fit, then allocates
remaining groups by proportional deficit. Presentation order is also seeded.
The default seed is `BibleQuoteBench/MCP/stratified-v1`; use distinct seeds to
sample different sets. These short diagnostic samples are not population estimates.

Translations of a reference stay together. With three editions, a case limit of
20 selects 18 cases (six complete reference groups); a limit below three is
rejected. Filter to one edition for an exact ten-case trial. A limit above the
available set selects all cases. Method, seed, cap, translation, and exact case
IDs are retained in the checkpoint and trial metadata. Older schema-one trials
resume their original dataset-prefix selection without being resampled.

The setup script accepts `-CaseLimit`, `-Translation`, and `-Seed` for defaults.
`-Preview` prints the entry without changing settings. For an explicit initial
trial, supply both `-Model` and `-RunIdPrefix`; ordinary trials use the tools and
need no configuration changes.

## Tools and retained artifacts

| Tool | Behavior |
| --- | --- |
| `begin_trial` | Starts a fresh trial with a run ID, explicit model label, and optional selection overrides. |
| `resume_trial` | Restores a saved trial and its original settings. |
| `benchmark_status` | Identity and progress; no reference text or correctness feedback. |
| `next_case` | Issues one edition-pinned prompt; repeats it until answered. |
| `submit_answer` | Saves raw output durably, including empty answers and whitespace. The same answer can be retried; changes are rejected. |
| `finish_run` | Requires all selected cases answered; writes results and returns aggregate metrics. Safe to repeat. |

Scoring feedback is withheld until every answer is immutable, including across
restarts. The server validates the corpus, excludes copy controls, locks the run
against concurrent writers, and binds resumed progress to the selected cases,
catalog, reference corpus, prompts, engine version, run ID, and model label.
Answers are limited to 16,384 Unicode characters; transport messages to 1 MiB.
There is no tool for reading arbitrary files, changing datasets, or running paid
providers. Client access to shell or other tools is outside this boundary.

Artifacts are saved under `results/mcp/run-<run_id>/`:

- `session.json`: authoritative checkpoint after each issued case and submitted
  answer; contains responses and dataset commitments, without reference text.
- `responses.jsonl` and `scores.jsonl`: existing benchmark record formats.
- `report.json` and `report.md`: descriptive metrics and translation-confusion
  reporting; the Markdown report explains the interactive evidence limitations.
- `trial.json`: evidence label, self-reported identity, dataset/prompt commitments,
  response and score hashes, and limitations. It is **not** a live-provider run
  manifest, and the controlled `analyze` workflow deliberately does not accept it.

Result files are generated at completion; interrupted trials retain their answers
in `session.json`. A partial export interrupted by filesystem failure can be
regenerated with `finish_run`. Local artifacts are not tamper-proof; the
commitments detect input drift, not deliberate edits by an evaluator. Results are
ignored by Git. You can feed the finished response file to `score` again or use
`summarize` and `report` with the score file, using the same selected dataset.

## Direct server command and transport

```console
cargo run --locked -- mcp --run-id local-trial --model assistant-label --case-limit 10 --translation bsb-2025-third-printing
```

Omit `--run-id` and `--model` to start idle. They must be supplied together for an
initial trial. `--seed` sets the selection default.

`--translations`, `--cases`, `--references`, and `--output-dir` are operator-selected
paths. Desktop launchers should use the built executable and absolute paths, as
the configuration script does. Reference text remains on the evaluator side.

The server implements the MCP stdio tools subset with newline-delimited JSON-RPC,
initialization/version negotiation, ping, `tools/list`, `tools/call`, notifications,
structured results for newer clients, and protocol/tool errors. Supported protocol
versions are `2024-11-05`, `2025-03-26`, `2025-06-18`, and `2025-11-25`. It advertises
only tools. It serves one active trial at a time and exits on stdin EOF; stdout contains
only protocol messages. HTTP hosting, browser clients, resources, prompts, and
server-initiated model sampling are outside this implementation.

Configuration references: [Codex MCP](https://developers.openai.com/codex/mcp),
[local servers and Claude Desktop](https://modelcontextprotocol.io/docs/develop/connect-local-servers),
[Cursor MCP](https://prod.cursor.com/help/customization/mcp). Protocol references:
[stdio](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports),
[lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle),
[tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).
