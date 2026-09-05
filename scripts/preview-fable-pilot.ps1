[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)

# Preparation only. This script never enables a policy or authorizes paid execution.
cargo run --locked -- run --provider anthropic --model claude-fable-5-1 `
    --run-id fable-validation-1 --reasoning-effort high --max-output-tokens 4096 `
    --translations data/pilot/v0.2/canonical/translations.json `
    --cases data/pilot/v0.2/canonical/cases.jsonl `
    --references data/pilot/v0.2/canonical/references.jsonl `
    --budget data/pilot/v0.2/fable-budget.example.json `
    --output results/fable-validation/responses.jsonl --dry-run
if ($LASTEXITCODE -ne 0) { throw 'Pilot preparation failed' }
