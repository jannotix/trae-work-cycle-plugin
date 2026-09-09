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

    $workflow = Get-Content (Join-Path $root '.github/workflows/release.yml') -Raw
    foreach ($required in @(
            'tools/sign-windows.ps1',
            'WINDOWS_CODE_SIGNING_CERTIFICATE_BASE64',
            'WINDOWS_CODE_SIGNING_CERTIFICATE_PASSWORD',
            '-RequireAuthenticode',
            'environment: windows-code-signing',
            'environment: production-release'
        )) {
        if ($workflow -notmatch [regex]::Escape($required)) {
            throw "release workflow is missing signing guard: $required"
        }
    }
    $signer = Get-Content (Join-Path $root 'tools/sign-windows.ps1') -Raw
    foreach ($required in @('/fd', 'SHA256', '/tr', '/td', 'verify', 'TimeStamperCertificate')) {
        if ($signer -notmatch [regex]::Escape($required)) {
            throw "Windows signing script is missing: $required"
        }
    }

    # A listing link to a mutable branch is a link that can start lying after
    # submission. The submission checklist requires the policy links to resolve
    # at the immutable release tag; this is that requirement, enforced.
    $manifest = Get-Content (Join-Path $root 'marketplace/manifest.json') -Raw | ConvertFrom-Json
    if (-not $manifest.version) { throw 'marketplace manifest is missing its version' }
    $expectedPrefix = "https://github.com/jannotix/trae-work-cycle-plugin/blob/v$($manifest.version)/"
    foreach ($field in @('security', 'privacy', 'support')) {
        $link = $manifest.$field
        if (-not $link) { throw "marketplace manifest is missing the '$field' link" }
        if (-not $link.StartsWith($expectedPrefix, [StringComparison]::Ordinal)) {
            throw "marketplace manifest '$field' link must resolve at the immutable tag v$($manifest.version), found: $link"
        }
    }

    Write-Host 'release guard tests passed'
} finally {
    if ((Test-Path -LiteralPath $fixture) -and $fixture.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
