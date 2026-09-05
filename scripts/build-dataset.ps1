[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot

$artifacts = @(
    @{
        Id = 'bsb-2025-third-printing'
        Name = 'bsb'
        Url = 'https://bereanbible.com/bsb_usfm.zip'
    },
    @{
        Id = 'asv-1901'
        Name = 'asv'
        Url = 'https://ebible.org/Scriptures/eng-asv_usfm.zip'
    },
    @{
        Id = 'web-classic-2020'
        Name = 'web'
        Url = 'https://ebible.org/Scriptures/eng-web_usfm.zip'
    }
)

New-Item -ItemType Directory -Force -Path 'data/sources', 'data/corpus', 'data/hidden' | Out-Null

$hiddenSeedPath = 'data/hidden/sampling-secret.txt'
if (-not (Test-Path -LiteralPath $hiddenSeedPath)) {
    $seedBytes = New-Object byte[] 32
    $random = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    try {
        $random.GetBytes($seedBytes)
    }
    finally {
        $random.Dispose()
    }
    [System.IO.File]::WriteAllText(
        (Join-Path $projectRoot $hiddenSeedPath),
        [Convert]::ToHexString($seedBytes).ToLowerInvariant()
    )
    Write-Output 'Created a private hidden-set seed. Back up data/hidden/sampling-secret.txt securely.'
}

foreach ($artifact in $artifacts) {
    $sourcePath = "data/sources/$($artifact.Name).zip"
    $corpusPath = "data/corpus/$($artifact.Name).jsonl"
    $lockPath = "data/locks/$($artifact.Name).json"
    $candidateLockPath = "data/sources/$($artifact.Name).candidate-lock.json"

    if (-not (Test-Path -LiteralPath $lockPath)) {
        throw "Missing committed provenance lock: $lockPath"
    }
    $expected = Get-Content -Raw -LiteralPath $lockPath | ConvertFrom-Json
    if (-not (Test-Path -LiteralPath $sourcePath)) {
        Invoke-WebRequest -Uri $artifact.Url -OutFile $sourcePath -UseBasicParsing
    }
    $actualSourceHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $sourcePath).Hash.ToLowerInvariant()
    if ($actualSourceHash -ne $expected.source_sha256) {
        throw "Source digest mismatch for $($artifact.Id): expected $($expected.source_sha256), got $actualSourceHash"
    }

    & cargo run --quiet -- import-usfm `
        --translation $artifact.Id `
        --source $sourcePath `
        --output $corpusPath `
        --lock-output $candidateLockPath
    if ($LASTEXITCODE -ne 0) {
        throw "USFM import failed for $($artifact.Id)"
    }
    $candidate = Get-Content -Raw -LiteralPath $candidateLockPath | ConvertFrom-Json
    if ($candidate.corpus_sha256 -ne $expected.corpus_sha256) {
        throw "Corpus digest mismatch for $($artifact.Id): expected $($expected.corpus_sha256), got $($candidate.corpus_sha256)"
    }
}

& cargo run --quiet -- sample `
    --corpus data/corpus/bsb.jsonl `
    --corpus data/corpus/asv.jsonl `
    --corpus data/corpus/web.jsonl `
    --lock data/locks/bsb.json `
    --lock data/locks/asv.json `
    --lock data/locks/web.json
if ($LASTEXITCODE -ne 0) {
    throw 'Dataset sampling failed'
}

& cargo run --quiet -- validate
if ($LASTEXITCODE -ne 0) {
    throw 'Public dataset validation failed'
}
& cargo run --quiet -- validate --cases data/hidden/cases.jsonl --references data/hidden/references.jsonl
if ($LASTEXITCODE -ne 0) {
    throw 'Hidden dataset validation failed'
}

Write-Output 'BibleQuoteBench v0.2 dataset reproduced successfully.'
