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
    [string] $ExpectedVersion = '',
    [switch] $RequireAuthenticode
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

function New-DeterministicZip([string] $Source, [string] $Destination) {
    Add-Type -AssemblyName System.IO.Compression
    $stream = [IO.File]::Open($Destination, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
        $archive = [IO.Compression.ZipArchive]::new($stream, [IO.Compression.ZipArchiveMode]::Create, $true)
        try {
            $stamp = [DateTimeOffset]::new(1980, 1, 1, 0, 0, 0, [TimeSpan]::Zero)
            foreach ($file in (Get-ChildItem -LiteralPath $Source -File | Sort-Object Name)) {
                $entry = $archive.CreateEntry($file.Name, [IO.Compression.CompressionLevel]::Optimal)
                $entry.LastWriteTime = $stamp
                $input = $file.OpenRead()
                $output = $entry.Open()
                try {
                    $input.CopyTo($output)
                } finally {
                    $output.Dispose()
                    $input.Dispose()
                }
            }
        } finally {
            $archive.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

function Assert-Authenticode([string] $Path) {
    $signature = Get-AuthenticodeSignature -FilePath $Path
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Authenticode signature is not valid for ${Path}: $($signature.Status)"
    }
    if (-not $signature.TimeStamperCertificate) {
        throw "Authenticode signature has no trusted timestamp for $Path"
    }
}

function Copy-LicenseMaterials([string] $Destination) {
    foreach ($name in @('LICENSE', 'NOTICE', 'README.md')) {
        Copy-Item -LiteralPath (Join-Path $root $name) -Destination (Join-Path $Destination $name)
    }
    & (Join-Path $root 'tools/package.ps1') -Root $root -NoticesOnly (Join-Path $Destination 'THIRD-PARTY-NOTICES.md')
    if ($LASTEXITCODE -ne 0) { throw 'failed to generate third-party notices' }
}

function Assert-LicenseMaterials([string] $Directory) {
    foreach ($name in @('LICENSE', 'NOTICE', 'README.md', 'THIRD-PARTY-NOTICES.md')) {
        if (-not (Test-Path -LiteralPath (Join-Path $Directory $name))) {
            throw "extracted runtime archive is missing $name"
        }
    }
}

try {
    if (Test-Path -LiteralPath $outputPath) { Remove-Item -LiteralPath $outputPath -Force }
    Copy-LicenseMaterials $stage
    if ($Platform -eq 'windows-x64') {
        if (-not $IsWindows) { throw 'the Windows runtime archive must be built on Windows' }
        if ($RequireAuthenticode) { Assert-Authenticode $binaryPath }
        $stagedBinary = Join-Path $stage 'trae-cycle.exe'
        Copy-Item -LiteralPath $binaryPath -Destination $stagedBinary
        New-DeterministicZip $stage $outputPath
        Expand-Archive -LiteralPath $outputPath -DestinationPath $extract
        $extractedBinary = Join-Path $extract 'trae-cycle.exe'
        if ($RequireAuthenticode) { Assert-Authenticode $extractedBinary }
    } else {
        if ($IsWindows) { throw 'the WSL runtime archive must be built on a Linux runner' }
        $stagedBinary = Join-Path $stage 'trae-cycle'
        Copy-Item -LiteralPath $binaryPath -Destination $stagedBinary
        & chmod 644 (Join-Path $stage 'LICENSE') (Join-Path $stage 'NOTICE') (Join-Path $stage 'README.md') (Join-Path $stage 'THIRD-PARTY-NOTICES.md')
        if ($LASTEXITCODE -ne 0) { throw 'failed to set WSL runtime document modes' }
        & chmod 755 $stagedBinary
        if ($LASTEXITCODE -ne 0) { throw 'failed to mark the WSL runtime executable' }
        & tar --sort=name '--mtime=@0' --owner=0 --group=0 --numeric-owner -C $stage -czf $outputPath 'LICENSE' 'NOTICE' 'README.md' 'THIRD-PARTY-NOTICES.md' 'trae-cycle'
        if ($LASTEXITCODE -ne 0) { throw 'failed to create the WSL runtime archive' }
        & tar -C $extract -xzf $outputPath
        if ($LASTEXITCODE -ne 0) { throw 'failed to extract the WSL runtime archive' }
        $extractedBinary = Join-Path $extract 'trae-cycle'
        & test -x $extractedBinary
        if ($LASTEXITCODE -ne 0) { throw 'the extracted WSL runtime is not executable' }
    }

    Assert-LicenseMaterials $extract
    & (Join-Path $root 'tools/smoke-runtime.ps1') -Binary $extractedBinary -ExpectedVersion $ExpectedVersion
    $digest = (Get-FileHash -LiteralPath $outputPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-Host "archive passed extraction smoke: $outputPath ($digest)"
} finally {
    if ((Test-Path -LiteralPath $workspace) -and $workspace.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $workspace -Recurse -Force
    }
}
