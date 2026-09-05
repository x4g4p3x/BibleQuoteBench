[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot

function Get-ArtifactHashes {
    param([string[]]$Paths)
    @($Paths | Sort-Object | ForEach-Object {
        $artifact = Get-FileHash -Algorithm SHA256 -LiteralPath $_
        "$($_):$($artifact.Hash)"
    })
}

function Invoke-PilotBuild {
    & cargo run --quiet --locked -- prepare-pilot --corpus data/corpus/bsb.jsonl --corpus data/corpus/asv.jsonl --corpus data/corpus/web.jsonl
    if ($LASTEXITCODE -ne 0) { throw 'Pilot preparation failed' }
    & cargo run --quiet --locked -- synthetic-pilot
    if ($LASTEXITCODE -ne 0) { throw 'Synthetic pilot failed' }
}

& ./scripts/build-dataset.ps1
$datasetPaths = @('data/dev/cases.jsonl', 'data/dev/references.jsonl', 'data/hidden/cases.jsonl', 'data/hidden/references.jsonl', 'data/release/v0.2-manifest.json')
$firstDataset = Get-ArtifactHashes $datasetPaths
& ./scripts/build-dataset.ps1
$secondDataset = Get-ArtifactHashes $datasetPaths
if (Compare-Object $firstDataset $secondDataset) { throw 'Dataset reconstruction was not byte-identical' }
Write-Output 'Dataset reconstruction is byte-identical across two builds.'

Invoke-PilotBuild
$pilotPaths = @(Get-ChildItem -LiteralPath 'data/pilot/v0.2', 'docs/pilot/v0.2' -Recurse -File | Select-Object -ExpandProperty FullName)
$firstPilot = Get-ArtifactHashes $pilotPaths
Invoke-PilotBuild
$secondPilotPaths = @(Get-ChildItem -LiteralPath 'data/pilot/v0.2', 'docs/pilot/v0.2' -Recurse -File | Select-Object -ExpandProperty FullName)
$secondPilot = Get-ArtifactHashes $secondPilotPaths
if (Compare-Object $firstPilot $secondPilot) { throw 'Pilot reconstruction was not byte-identical' }
Write-Output 'Pilot datasets, synthetic responses, manifests, and analyses are byte-identical across two builds.'
Write-Output 'Release artifacts prepared. No model API calls were made.'
