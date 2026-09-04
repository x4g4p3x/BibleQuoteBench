[CmdletBinding()]
param(
    [switch]$Coverage
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot

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
