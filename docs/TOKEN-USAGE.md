# Subscription clients and token accounting

Report the execution client alongside every model name, score and usage figure.
The subscription trials use these paths:

| Requested model | Answering client | Benchmark interaction | Tool evidence |
| --- | --- | --- | --- |
| Grok 4.7 xHigh, Fast off | Native Cursor CLI, Cursor subscription login | MCP status, case retrieval and answer submission in a fresh conversation for each case | Client permissions and live transcript checks; upstream model/tool configuration unverified |
| Claude Opus 5.5 Max, Fast off | Native Cursor CLI, Cursor subscription login | Same benchmark MCP tools and fresh conversations | Client permissions and live transcript checks; upstream model/tool configuration unverified |
| GPT-6.1 Sol Max | Native Codex CLI, ChatGPT subscription login | Restricted MCP runner orchestrates a canonical single-prompt native request for each case | Authoritative request gate verifies the model, effort, empty tool schema, disabled tool choice and no conversation history |

Codex desktop coordinates the work, validation and backups. Its coordination
tokens are separate from the answering model's benchmark usage. The Sol run is
therefore described as **Codex CLI via the restricted subscription runner**;
the Cursor runs are described as **Cursor CLI via benchmark MCP tools**.

The operator supplies no separate paid provider API keys and makes no separate
paid provider API calls. Native subscription clients still make network model
requests and consume account usage. “No API quota” must not be described as
“no network requests” or “no tokens.”

## What each report retains

Private run reports retain input, output, cache-read and cache-write counters
where available, separate reasoning counters where available, the number of
accepted answers, attempts with usage, and attempts without usage. Missing
usage is **unknown**, never zero. Usage includes known retries and interrupted
attempts that have counters. Unknown interrupted usage remains explicitly
missing even after the benchmark finishes.

Cursor final-result counters and operator-supplied account exports are separate
views of the same activity. Match the four native counter values against export
rows as a multiset, so duplicate rows cannot be counted twice. Verify that each
export's total equals its four category values. Select the requested model and
the audited run window, excluding earlier pilot runs. Report additional account
rows separately; timestamps alone cannot assign a row to a particular case or
prove that unrelated account activity is absent. Retain the export's SHA-256,
UTC window, snapshot time and operator attribution, rather than its raw rows,
account identifiers or billing costs. Never add the account total to the native
total.

For the Codex request gate, verify each native response stream's SHA-256 against
the preserved gate audit. Extract only the usage fields from its completed
response and cross-check input/output values against the execution summary.
OpenAI usage counts include cached tokens within input tokens and reasoning
tokens within output tokens; adding those subsets again would double-count.
See [OpenAI's usage documentation](https://developers.openai.com/api/docs/guides/agents-api/observability).

The private evaluator contains the reproducible aggregate-only reporter and
tests. Raw exports, transcripts, thinking, credentials and local machine paths
remain outside Git. Token reports contain no answer text.

## Comparison limits

These results measure a **model and client pipeline**. Cursor MCP conversations
include instructions, tool schemas, tool results, conversation history and
possible reconnect work. The restricted Codex path sends a much smaller,
tool-free canonical request. Tokenizers, cache policies and reasoning budgets
also differ across models. Highest available effort does not establish an equal
reasoning budget. Token totals are useful operational measurements, but cannot
establish intrinsic model efficiency or a controlled model ranking.

Keep accepted-answer coverage and usage coverage next to the totals. A final
300-answer score can coexist with incomplete usage. An account export captured
before completion remains a partial snapshot; it cannot become a final total
merely because the run subsequently finishes. Report costs only from a clearly
scoped, verified billing source, rather than estimating them from subscription
token counters.
