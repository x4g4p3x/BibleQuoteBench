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
  --run-id MODEL_ID-run-1 --budget results/approved-budget.json --allow-paid --output results/responses.jsonl
```

`--case-limit` supports small smoke runs. `--base-url` and `--api-key-env`
override defaults without placing secrets in command arguments or output files.
The local OpenAI-compatible default is `http://localhost:11434/v1`.

Recall requests supply no tools, retrieval, search, prior conversation, or reference
text. The separate `copy_control` track explicitly includes supplied passage text.
OpenAI and xAI requests explicitly set `store=false` and provide an empty
tool list. Provider request IDs, resolved model identifiers, OpenAI-compatible
system fingerprints, requested temperature, and per-case errors are retained.

The Responses and Anthropic adapters accept `--reasoning-effort` and retain the
exact requested setting. Anthropic sends it as `output_config.effort`; unsupported
effort names and non-default Fable temperature settings fail locally. Other adapters reject
that flag rather than silently dropping it. Verify model support before using it; settings are never automatically
substituted. Existing runs require `--resume`, which verifies the checkpoint
before continuing. See [budgeted execution](EXECUTION.md) for spending controls,
interruption recovery, usage accounting, and the offline Fable pilot preview.

Every successful run command writes `NAME.manifest.json` next to `NAME.jsonl`.
It binds all intended cases even when `--case-limit` produces only a subset.
Analyze repeated complete runs together using repeated `--responses` arguments:

```console
cargo run -- analyze --responses results/model-run-1.jsonl --responses results/model-run-2.jsonl --output-dir results/analysis
```

Supply the same `--translations`, `--cases`, and `--references` paths used by
the run when evaluating diagnostic or hidden data. Legacy response files remain
scoreable but cannot become validated comparisons without original run manifests.

Temperature is optional because not every current model accepts it. Use
`--temperature 0` when the selected model supports that deterministic setting;
otherwise omit the flag and document the provider's effective behavior.

API-backed tests validate serialization and response extraction without making
paid network calls. A real run therefore requires the relevant credential and
incurs the provider's normal usage costs.

## Held GPT-6 pilot

The requested target is GPT-6 Astra with `max` reasoning with a EUR 20 ceiling, but the user has
explicitly withheld permission to spend. `data/pilot/v0.2/live-plan.json` records
that hold and zero spend. Neither pilot preparation nor `synthetic-pilot` calls
a model API, and there is deliberately no command that executes this held plan.

As checked on 2026-09-05, the [official model page](https://developers.openai.com/api/docs/models/gpt-6-astra)
documents `gpt-6-astra` and reasoning levels `low`, `medium`, `high`, `xhigh`, and
`max`. The saved plan uses the user-selected `max` level. The [model guide](https://developers.openai.com/api/docs/guides/latest-model)
also says to omit `temperature` for Astra. Account access has not been verified.
Before any later paid execution, verify pricing and provide an explicitly
approved, enabled budget policy. The generic `run` command now requires that
policy and `--allow-paid` for live endpoints. The saved GPT-6 plan remains a held
planning artifact; it is not an enabled budget policy.
