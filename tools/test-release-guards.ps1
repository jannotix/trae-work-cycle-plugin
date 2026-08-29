#requires -Version 7.0
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$fixture = Join-Path $tempRoot ("trae-release-guard-{0}" -f [Guid]::NewGuid().ToString('N'))
$fixture = [IO.Path]::GetFullPath($fixture)
if (-not $fixture.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'release guard fixture escaped the system temporary directory'
}
New-Item -ItemType Directory -Path $fixture | Out-Null

try {
    [IO.File]::WriteAllText((Join-Path $fixture 'clean.txt'), 'no credential material')
    & (Join-Path $root 'tools/scan-release-secrets.ps1') -Path $fixture
    [IO.File]::WriteAllText((Join-Path $fixture 'secret.txt'), 'token=sk-abcdefghijklmnop1234567890')
    $rejected = $false
    try {
        & (Join-Path $root 'tools/scan-release-secrets.ps1') -Path $fixture
    } catch {
        $rejected = $_.Exception.Message -match 'OpenAI-style API key|secret scan failed'
    }
    if (-not $rejected) { throw 'release secret scan accepted a seeded credential' }
    Write-Host 'release guard tests passed'
} finally {
    if ((Test-Path -LiteralPath $fixture) -and $fixture.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
