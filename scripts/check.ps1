[CmdletBinding()]
param(
    [switch]$Coverage,
    [switch]$RetentionChild
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot

if (-not $RetentionChild) {
    $helper = Join-Path $PSScriptRoot 'ci/ci_artifact_retention.py'
    $powerShell = (Get-Process -Id $PID).Path
    $child = @($powerShell, '-NoProfile', '-File', $PSCommandPath, '-RetentionChild')
    if ($Coverage) { $child += '-Coverage' }
    & python $helper @child
    exit $LASTEXITCODE
}

function Invoke-QualityCommand {
    param(
        [Parameter(Mandatory)]
        [string]$Description,
        [Parameter(Mandatory)]
        [scriptblock]$Command
    )

    Write-Output "==> $Description"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE"
    }
}

Invoke-QualityCommand 'CI artifact retention tests' {
    python -m unittest discover -s scripts/tests -p test_ci_artifact_retention.py
}
Invoke-QualityCommand 'Formatting' { cargo fmt --check }
Invoke-QualityCommand 'Compilation' { cargo check --locked --all-targets --all-features }
Invoke-QualityCommand 'Clippy' { cargo clippy --locked --all-targets --all-features -- -D warnings }
Invoke-QualityCommand 'Tests' { cargo test --locked --all-targets --all-features }
Invoke-QualityCommand 'Rust documentation' {
    $env:RUSTDOCFLAGS = '-D warnings'
    cargo doc --locked --no-deps --all-features
}
Invoke-QualityCommand 'Publication guard' { cargo run --quiet --locked -- guard-tracked }

if ($Coverage) {
    Invoke-QualityCommand 'Line coverage (minimum 80%)' {
        cargo llvm-cov --locked --all-targets --all-features --fail-under-lines 80 --summary-only
    }
}

Write-Output 'All requested quality gates passed.'
