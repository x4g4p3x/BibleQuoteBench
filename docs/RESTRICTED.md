# Restricted subscription recall

Use this mode when the answering model must receive no tools, copied answers,
conversation history, or reference corpus. It uses a ChatGPT subscription through
the installed Codex CLI, without a provider API key or API billing. Subscription
usage limits still apply, and prompts leave the machine for the subscription service.

Codex desktop, Claude Desktop, and Cursor can all orchestrate the trial over MCP.
**The answering model is the operator-selected Codex model in every client.** This
does not measure Claude or Cursor's selected model. The ordinary [interactive
server](MCP.md) remains available for those models, with weaker isolation.

## Start a trial

Install a current native Codex CLI and sign in with ChatGPT. The runner requires
file-backed subscription credentials in `CODEX_HOME/auth.json` (normally
`~/.codex/auth.json`); keyring-only and API-key logins are rejected. The CLI must
support `--ignore-user-config`, `--ephemeral`, and `--strict-config`.

Run ten public BSB cases directly:

```powershell
cargo run --locked -- restricted --model gpt-6.1-sol --run-id recall-01 --case-limit 10
```

The model must be available to the signed-in subscription. Selection uses the
same recorded stratified sampling as interactive trials. `--translation`,
`--seed`, `--codex-bin`, `--output-dir`, and `--timeout-seconds` are configurable.
The default per-case timeout is 120 seconds; allowed values are 1–300 seconds.
There is no fallback to another model or paid API endpoint.

To resume, repeat the original arguments and add `--resume`. A completed run makes
no further model requests. Failed, timed-out, or interrupted attempts remain
blocked: their answers and subscription usage may be uncertain. Preserve those
artifacts and use a fresh run ID for a new trial. Do not present a restarted
trial as a continuation with missing failures omitted. Updating the Codex binary,
model, timeout, instructions, selection, or dataset requires a new run ID.

## Connect desktop clients

Build and register a separate restricted server:

```powershell
cargo build --locked --release
./scripts/configure-mcp.ps1 -Client codex,claude,cursor -RestrictedModel gpt-6.1-sol
```

This adds `biblequotebench-restricted` and preserves the ordinary `biblequotebench`
entry. Existing JSON settings are backed up. `-Preview` shows the registration
without editing settings. Restart the clients or reload MCP connections.

Ask the orchestrating assistant:

> Use biblequotebench-restricted. Begin trial `recall-01-codex` with model
> `gpt-6.1-sol` and case_limit 10. Repeatedly call next_case and pass its case_id
> to answer_case until complete, then call finish_run and report the summary.

`next_case` returns a case ID and progress. The actual prompt stays inside the
gate. `answer_case` accepts only that ID; supplying an output, extra instructions,
or other context is rejected. Manual `submit_answer` is disabled. The model
cannot be changed by MCP callers, and callers cannot exceed the operator's case
cap (ten by default). A repeated accepted answer is idempotent; an uncertain
attempt cannot be replayed. Scores remain withheld until all answers are fixed.

## Enforcement and retained evidence

For each case the evaluator starts the trusted native CLI in an empty temporary
workspace with a private temporary Codex home. User configuration, MCP servers,
plugins, repository instructions, API-key environment variables, and prior
conversation state are excluded. Retrieval features are also disabled.

A loopback Responses gate receives the CLI's request and replaces the entire
body with the canonical edition-pinned prompt and fixed recall instructions.
The outgoing request has `tools: []`, `tool_choice: "none"`,
`parallel_tool_calls: false`, no history, and low reasoning effort. Reference
text is never included. The gate forwards only authentication/protocol headers
to the fixed Codex subscription endpoint; HTTP redirects and ambient proxies
are disabled. One local model-catalogue query is answered without an upstream
request. Exactly one generation is permitted per case.

The gate buffers the complete service response before delivering any bytes to
the CLI. Tool calls, unknown output/event types, malformed or incomplete streams,
model mismatches, missing usage, and duplicate completions cause rejection. The
request limit is 2 MiB, response limit 8 MiB, and answer limit 16,384 Unicode
characters. Child processes are stopped and waited for on failure or timeout.
Attempts are checkpointed before launch and never automatically retried.

Each trial retains the normal responses, scores, and reports, plus:

- schema-three `restricted_codex` identity with executable and instruction hashes;
- per-case accepted or blocked attempt records;
- canonical `gate/case-N/request.json` and raw `response.sse` audits;
- response model identity, token usage, and typed refusals.

Accepted answers and hashes are checked against their audits during resume.
Authentication headers and tokens are absent from audit files. Raw service
responses can contain encrypted reasoning and other subscription metadata;
keep the ignored `results/` directory private. Temporary credentials are removed
after the case. Native token refreshes are retained atomically when the source
credentials still match; cooperating benchmark runners serialize refresh writes.
Avoid changing accounts or signing in concurrently during a trial.

## Scope and compatibility

This enforces the request and output boundary for the answering model. It cannot
disable unrelated tools in the orchestrating desktop chat. Browsing by that chat
does not provide a route to submit its findings as an answer or model context.
The local evaluator, native CLI, subscription service, and operator remain
trusted. Local files are not tamper-proof proof of service-side behavior.

Results have descriptive reports and are kept separate from `interactive_mcp`
and controlled provider manifests. `analyze` does not pool these trials with
controlled provider runs. Subscription service or CLI protocol changes may
require an update; unexpected behavior fails closed.

CLI-only restrictions were considered: Cursor's print/ask modes do not provide a
zero-tool boundary, and installing newer CLI flags cannot let an MCP server
control other tools in the host. MCP sampling also depends on host implementation
and context policy. The authoritative request gate therefore supplies the
enforcement, with CLI isolation as defense in depth. The documented [Codex
provider configuration](https://learn.chatgpt.com/docs/config-file/config-reference)
supports subscription authentication with a custom Responses endpoint. The
subscription transport itself is version-sensitive. See also the [Codex CLI
reference](https://learn.chatgpt.com/docs/cli/reference) and [Cursor CLI
parameters](https://cursor.com/docs/cli/reference/parameters).
