# Provider execution

The `run` command supports five adapters:

| Adapter | API | Default credential |
| --- | --- | --- |
| `openai` | Responses API | `OPENAI_API_KEY` |
| `anthropic` | Messages API | `ANTHROPIC_API_KEY` |
| `gemini` | `generateContent` | `GEMINI_API_KEY` |
| `xai` | Responses API | `XAI_API_KEY` |
| `openai-compatible` | Chat Completions | none |

Example:

```console
cargo run --release -- run --provider openai --model MODEL_ID \
  --run-id MODEL_ID-run-1 --temperature 0 --output results/responses.jsonl
```

`--case-limit` supports small smoke runs. `--base-url` and `--api-key-env`
override defaults without placing secrets in command arguments or output files.
The local OpenAI-compatible default is `http://localhost:11434/v1`.

No adapter supplies tools, retrieval, search, prior conversation, or reference
text. OpenAI and xAI requests explicitly set `store=false` and provide an empty
tool list. Provider request IDs, resolved model identifiers, OpenAI-compatible
system fingerprints, requested temperature, and per-case errors are retained.

Temperature is optional because not every current model accepts it. Use
`--temperature 0` when the selected model supports that deterministic setting;
otherwise omit the flag and document the provider's effective behavior.

API-backed tests validate serialization and response extraction without making
paid network calls. A real run therefore requires the relevant credential and
incurs the provider's normal usage costs.
