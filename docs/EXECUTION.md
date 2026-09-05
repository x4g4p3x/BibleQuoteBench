# Budgeted and resumable execution

The CLI now saves each request's reservation before contacting a provider and
saves each response before starting the next request. Live endpoints require a
budget policy with `execution_enabled: true` and the explicit `--allow-paid` flag.
Plain HTTP is limited to loopback tests. Requests do not follow redirects.

## Prepared pilot: spending remains paused

The Fable validation pilot is prepared for one complete canonical run: 12 shared
references across three editions, 36 requests. It uses `claude-fable-5-1`, explicit
`high` effort, and a 4,096-token total output limit. No credentials or paid calls
are required to preview it:

```powershell
./scripts/preview-fable-pilot.ps1
```

The example policy is disabled. Its EUR 5 ceiling is a proposed planning example,
not spending authorization. The earlier GPT-6 Astra max plan also remains held.
The preview checks the actual dataset, parameters, and conservative reservations;
it writes no run artifacts and contacts no model provider. A complete worst-case
run can exceed EUR 5, so the executor may stop with partial progress if usage is
high. Do not reduce the token limit merely to force the run into that ceiling.

Before a later authorized pilot, verify account access and current pricing, copy
the policy into `results/`, set the approved ceiling and prices, and enable it.
Use the same arguments as the preview, with the copied policy, replacing
`--dry-run` with `--allow-paid`. The credential remains in `ANTHROPIC_API_KEY`.
Never place a credential in the policy or command text.

## Cost accounting

Policy amounts are integer nano-euros: 1 EUR = 1,000,000,000 nano-EUR. Rates are
nano-euros per token. The Fable example reserves EUR 10 per million input tokens
and EUR 50 per million output tokens. This uses conservative EUR/USD parity for
the documented USD 10/50 prices; taxes and payment fees are excluded. Price
assumptions and their verification date are part of the immutable campaign policy.

Each request reserves the prompt's UTF-8 byte length plus 4,096 framing tokens,
and the configured maximum output tokens, at those rates. This deliberately
overestimates these short, text-only prompts. No paid tools, prompt-cache writes,
or hidden multi-turn histories are requested. A request starts only if its full
reservation fits in the remaining campaign allowance. The ceiling applies to
the configured prices and bounds; it is not a provider-side invoice cap.

Reported token usage settles the reservation after a response. Output totals
include thinking without double-counting it. Missing usage, interrupted requests,
and uncertain failures retain the full reservation. If reported cost exceeds its
reservation, execution stops after saving that result for investigation. The HTML
report shows usage, accounted cost, and uncertain billing separately. Accounted
cost is an estimate from policy prices, not a reconciled invoice.

All runs using the same policy file share its adjacent `.ledger.json` checkpoint
and spending ceiling. Use unique run IDs. Keep the policy and checkpoint together;
creating a different policy file starts a different campaign, not an extension of
the original allowance. Do not edit or remove an active campaign's checkpoint.

## Interruption and recovery

Append `--resume` to the original execution command. Dataset, prompts, model,
sampling settings, output limit, case limit, output path, and budget policy must
match the checkpoint. A completed run can be resumed to restore its output file
without contacting the provider.

Checkpoints are replaced atomically after flushing their contents to disk. OS file
locks exclude concurrent writers and are released when a process exits or is
killed. The checkpoint is authoritative; response JSONL is rebuilt from it when
resuming. A final manifest is written only when the intended run loop finishes.
A case-limited run still cannot pass full-dataset analysis.

If a request was in flight when the process stopped, its outcome is unknown.
Resume records an explicit operational failure, retains its reservation, and
continues with the next case. It never silently resends that request. HTTP and
transport failures are also recorded without automatic retries. Any deliberate
rerun needs a new run ID and must fit in the same campaign allowance.

`--fail-fast` saves the failure and budget state before stopping. Resume continues
after that saved case. Tests cover an actual process kill during a loopback
request, recovery after a damaged output export, budget exhaustion, unknown usage,
shared campaign spending, configuration drift, and concurrent lock exclusion.

## Completion limits and interpretation

Responses retain provider stop reasons and token usage. Token-limit cutoffs have
a separate `truncated` classification and cannot count as exact recall, even if
the visible prefix happens to match. They remain in end-to-end recall and in
recall conditional on provider success, and are excluded from output stability.
Reports expose them as **Token-limit cutoff** in outcome counts and edition
diagnostics. Older response files without execution metadata remain readable.

The default output limit is now 4,096 total tokens. Claude effort is sent as
`output_config.effort`; Fable accepts temperature 1.0 or omission and rejects other values locally. A tiny live
pilot should establish actual token use, cutoff frequency, and latency before
choosing settings for a full study. Batch execution remains future work.

Provider references: [Fable specifications](https://platform.claude.com/docs/en/models/fable-5-1/overview),
[effort](https://platform.claude.com/docs/en/build-with-claude/effort), and
[thinking usage](https://platform.claude.com/docs/en/build-with-claude/thinking).
