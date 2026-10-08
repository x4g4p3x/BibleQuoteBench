#requires -Version 7.0
[CmdletBinding()]
param(
    [ValidateSet('codex', 'claude', 'cursor')]
    [string[]]$Client = @('codex'),
    [string]$Model,
    # Optional fixed Codex model for server-side, tool-free subscription recall.
    [ValidatePattern('^[A-Za-z0-9_.-]{1,256}$')]
    [string]$RestrictedModel,
    [string]$CodexBin = 'codex',
    [ValidatePattern('^[A-Za-z0-9_-]{1,48}$')]
    [string]$RunIdPrefix,
    [ValidateRange(1, 100000)]
    [int]$CaseLimit = 10,
    [string]$Translation = 'bsb-2025-third-printing',
    [ValidateNotNullOrEmpty()]
    [string]$Seed = 'BibleQuoteBench/MCP/stratified-v1',
    # Optional alternate JSON config path, for a single Claude or Cursor client.
    [string]$ConfigPath,
    # Print the new entry and destination without editing app settings.
    [switch]$Preview
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$executableName = if ($IsWindows) { 'biblequotebench.exe' } else { 'biblequotebench' }
$executablePath = Join-Path $projectRoot "target/release/$executableName"
if (-not (Test-Path -LiteralPath $executablePath)) {
    throw 'Build the server first with: cargo build --locked --release'
}
if ($ConfigPath -and ($Client.Count -ne 1 -or $Client[0] -eq 'codex')) {
    throw '-ConfigPath requires one Claude or Cursor client.'
}
if ($Model -and [string]::IsNullOrWhiteSpace($Model)) {
    throw '-Model must be a nonempty label.'
}
if ([bool]$Model -ne [bool]$RunIdPrefix) {
    throw '-Model and -RunIdPrefix must be supplied together for an initial trial. Omit both to start trials through MCP tools.'
}
if ($RestrictedModel -and $Model -and $RestrictedModel -ne $Model) {
    throw '-Model must match -RestrictedModel for an initial restricted trial.'
}
$serverName = if ($RestrictedModel) { 'biblequotebench-restricted' } else { 'biblequotebench' }
if ($RestrictedModel) {
    $CodexBin = (Get-Command $CodexBin -CommandType Application -ErrorAction Stop).Source
}

foreach ($clientName in $Client) {
    $serverArguments = @(
        'mcp', '--case-limit', "$CaseLimit", '--translation', $Translation, '--seed', $Seed,
        '--translations', (Join-Path $projectRoot 'data/dev/translations.json'),
        '--cases', (Join-Path $projectRoot 'data/dev/cases.jsonl'),
        '--references', (Join-Path $projectRoot 'data/dev/references.jsonl'),
        '--output-dir', (Join-Path $projectRoot 'results/mcp')
    )
    if ($RunIdPrefix) {
        $serverArguments += @('--run-id', "$RunIdPrefix-$clientName", '--model', $Model)
    }
    if ($RestrictedModel) {
        $serverArguments += @('--restricted-model', $RestrictedModel, '--codex-bin', $CodexBin)
    }
    $entry = @{ command = $executablePath; args = $serverArguments }
    if ($clientName -eq 'codex') {
        if ($Preview) {
            Write-Output "Codex: server $serverName in the shared Codex configuration"
            $entry | ConvertTo-Json -Depth 10
            continue
        }
        & codex mcp add $serverName -- $executablePath @serverArguments
        if ($LASTEXITCODE -ne 0) { throw 'Codex MCP registration failed.' }
        Write-Output 'Configured BibleQuoteBench for Codex. Reload its MCP connections or restart the app.'
        continue
    }
    $destination = if ($ConfigPath) {
        [System.IO.Path]::GetFullPath($ConfigPath)
    } elseif ($clientName -eq 'cursor') {
        Join-Path ([Environment]::GetFolderPath('UserProfile')) '.cursor/mcp.json'
    } elseif ($IsWindows) {
        $storeConfigs = @(Get-ChildItem -LiteralPath (Join-Path $env:LOCALAPPDATA 'Packages') -Directory -Filter 'Claude_*' -ErrorAction SilentlyContinue |
            ForEach-Object { Join-Path $_.FullName 'LocalCache/Roaming/Claude/claude_desktop_config.json' } |
            Where-Object { Test-Path -LiteralPath (Split-Path -Parent $_) })
        if ($storeConfigs.Count -gt 1) { throw 'Multiple Claude Store profiles found; choose one with -ConfigPath.' }
        if ($storeConfigs.Count -eq 1) { $storeConfigs[0] } else {
            Join-Path $env:APPDATA 'Claude/claude_desktop_config.json'
        }
    } elseif ($IsMacOS) {
        Join-Path ([Environment]::GetFolderPath('UserProfile')) 'Library/Application Support/Claude/claude_desktop_config.json'
    } else {
        throw 'Use -ConfigPath to specify the Claude Desktop configuration location.'
    }
    if ($Preview) {
        Write-Output "$clientName`: $destination"
        @{ mcpServers = @{ $serverName = $entry } } | ConvertTo-Json -Depth 10
        continue
    }
    $settings = if (Test-Path -LiteralPath $destination) {
        Get-Content -LiteralPath $destination -Raw | ConvertFrom-Json -AsHashtable
    } else { @{} }
    if ($settings -isnot [System.Collections.IDictionary]) { throw 'App configuration must be a JSON object.' }
    if (-not $settings.Contains('mcpServers')) { $settings['mcpServers'] = @{} }
    if ($settings['mcpServers'] -isnot [System.Collections.IDictionary]) { throw 'mcpServers must be a JSON object.' }
    $settings['mcpServers'][$serverName] = $entry
    $parentDirectory = Split-Path -Parent $destination
    [System.IO.Directory]::CreateDirectory($parentDirectory) | Out-Null
    if (Test-Path -LiteralPath $destination) {
        $backup = "$destination.bqb-backup.$(Get-Date -Format 'yyyyMMdd-HHmmss-fffffff')"
        Copy-Item -LiteralPath $destination -Destination $backup
        Write-Output "Saved original configuration: $backup"
    }
    $temporary = "$destination.bqb-tmp"
    [System.IO.File]::WriteAllText($temporary, ($settings | ConvertTo-Json -Depth 100))
    Move-Item -LiteralPath $temporary -Destination $destination -Force
    Write-Output "Configured BibleQuoteBench for $clientName at $destination. Restart the app."
}
