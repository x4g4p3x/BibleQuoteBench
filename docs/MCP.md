# Interactive trials over MCP

The local MCP server lets the assistant in Codex desktop, Claude Desktop, or
Cursor answer benchmark prompts and retain scored results without the benchmark
calling a model API. It needs no API key and makes no network requests. The
assistant still uses the client app's normal subscription or usage allowance.

These are **interactive trials**, labeled `interactive_mcp`, with self-reported
model identity. Shared conversation context, client instructions, unknown sampling
settings, and access to other tools prevent treating these as controlled
closed-book provider runs. Start a fresh conversation and disable retrieval tools
where possible. Use the public development data for these trials; any case sent
to an assistant becomes part of that conversation and may leave the local machine.
The server does not expose reference text or use MCP sampling.

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

The defaults select ten BSB cases and create distinct run IDs
`interactive-01-codex`, `interactive-01-claude`, and `interactive-01-cursor`.
Model labels initially say `model unspecified`. To record a particular model and
start a new trial, configure that client with the label shown in its model picker:

```powershell
./scripts/configure-mcp.ps1 -Client codex -Model 'your actual model label' -RunIdPrefix trial-02 -CaseLimit 20
```

`-Translation asv-1901` or `-Translation web-classic-2020` selects another edition.
Selection follows dataset order; filtering precedes the case limit. A limit larger
than the available set selects the entire set. Use a different `-RunIdPrefix` for
each repetition or model change. Reusing identical inputs resumes the existing
trial. `-Preview` prints the new entry without changing settings.

Use a fresh chat and ask the assistant:

> Use the BibleQuoteBench MCP tools to complete this interactive trial. Call
> benchmark_status, then next_case. Answer each prompt from internal recall only,
> without browsing, retrieval, Bible APIs, or inspecting local files. Send only the
> exact passage text as output to submit_answer, using the issued case_id. Do not
> revise answers. Repeat until complete, call finish_run, and report the summary.

## Tools and retained artifacts

| Tool | Behavior |
| --- | --- |
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

`--translations`, `--cases`, `--references`, and `--output-dir` are operator-selected
paths. Desktop launchers should use the built executable and absolute paths, as
the configuration script does. Reference text remains on the evaluator side.

The server implements the MCP stdio tools subset with newline-delimited JSON-RPC,
initialization/version negotiation, ping, `tools/list`, `tools/call`, notifications,
structured results for newer clients, and protocol/tool errors. Supported protocol
versions are `2024-11-05`, `2025-03-26`, `2025-06-18`, and `2025-11-25`. It advertises
only tools. It serves one run per process and exits on stdin EOF; stdout contains
only protocol messages. HTTP hosting, browser clients, resources, prompts, and
server-initiated model sampling are outside this implementation.

Configuration references: [Codex MCP](https://developers.openai.com/codex/mcp),
[local servers and Claude Desktop](https://modelcontextprotocol.io/docs/develop/connect-local-servers),
[Cursor MCP](https://prod.cursor.com/help/customization/mcp). Protocol references:
[stdio](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports),
[lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle),
[tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).
