#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Binary,
    [Parameter(Mandatory = $true)]
    [ValidateSet('windows-x64', 'wsl-x64')]
    [string] $Platform,
    [Parameter(Mandatory = $true)]
    [string] $Output,
    [string] $ExpectedVersion = ''
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$binaryPath = (Resolve-Path $Binary).Path
$cargo = Get-Content (Join-Path $root 'production/Cargo.toml') -Raw
if (-not $ExpectedVersion) {
    if ($cargo -notmatch '(?s)\[workspace\.package\].*?version\s*=\s*"([^"]+)"') {
        throw 'workspace version not found'
    }
    $ExpectedVersion = $Matches[1]
}
$outputPath = [IO.Path]::GetFullPath($Output)
$outputParent = Split-Path $outputPath -Parent
if (-not (Test-Path $outputParent)) { New-Item -ItemType Directory -Path $outputParent | Out-Null }

$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$workspace = Join-Path $tempRoot ("trae-cycle-archive-{0}" -f [Guid]::NewGuid().ToString('N'))
$workspace = [IO.Path]::GetFullPath($workspace)
if (-not $workspace.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'archive workspace escaped the system temporary directory'
}
$stage = Join-Path $workspace 'stage'
$extract = Join-Path $workspace 'extract'
New-Item -ItemType Directory -Path $stage, $extract | Out-Null

try {
    if (Test-Path -LiteralPath $outputPath) { Remove-Item -LiteralPath $outputPath -Force }
    if ($Platform -eq 'windows-x64') {
        if (-not $IsWindows) { throw 'the Windows runtime archive must be built on Windows' }
        $stagedBinary = Join-Path $stage 'trae-cycle.exe'
        Copy-Item -LiteralPath $binaryPath -Destination $stagedBinary
        Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $outputPath
        Expand-Archive -LiteralPath $outputPath -DestinationPath $extract
        $extractedBinary = Join-Path $extract 'trae-cycle.exe'
    } else {
        if ($IsWindows) { throw 'the WSL runtime archive must be built on a Linux runner' }
        $stagedBinary = Join-Path $stage 'trae-cycle'
        Copy-Item -LiteralPath $binaryPath -Destination $stagedBinary
        & chmod 755 $stagedBinary
        if ($LASTEXITCODE -ne 0) { throw 'failed to mark the WSL runtime executable' }
        & tar -C $stage -czf $outputPath 'trae-cycle'
        if ($LASTEXITCODE -ne 0) { throw 'failed to create the WSL runtime archive' }
        & tar -C $extract -xzf $outputPath
        if ($LASTEXITCODE -ne 0) { throw 'failed to extract the WSL runtime archive' }
        $extractedBinary = Join-Path $extract 'trae-cycle'
        & test -x $extractedBinary
        if ($LASTEXITCODE -ne 0) { throw 'the extracted WSL runtime is not executable' }
    }

    & (Join-Path $root 'tools/smoke-runtime.ps1') -Binary $extractedBinary -ExpectedVersion $ExpectedVersion
    $digest = (Get-FileHash -LiteralPath $outputPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-Host "archive passed extraction smoke: $outputPath ($digest)"
} finally {
    if ((Test-Path -LiteralPath $workspace) -and $workspace.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $workspace -Recurse -Force
    }
}
