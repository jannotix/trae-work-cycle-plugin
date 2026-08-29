#requires -Version 7.0
[CmdletBinding()]
param(
    [string] $Root = (Split-Path $PSScriptRoot -Parent),
    [string] $ExpectedSha = '',
    [string] $RefName = ''
)

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path $Root).Path
$status = @(& git -C $Root status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0) { throw 'release context requires a readable git repository' }
if ($status.Count -gt 0) {
    throw "release context requires a clean clone: $(($status | Select-Object -First 20) -join '; ')"
}

$revision = (& git -C $Root rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $revision -notmatch '^[0-9a-f]{40}$') {
    throw 'release context could not resolve an exact full Git SHA'
}
if ($ExpectedSha -and $revision -ne $ExpectedSha.ToLowerInvariant()) {
    throw "checked-out revision $revision does not match expected SHA $ExpectedSha"
}

$cargo = Get-Content (Join-Path $Root 'production/Cargo.toml') -Raw
if ($cargo -notmatch '(?s)\[workspace\.package\].*?version\s*=\s*"([^"]+)"') {
    throw 'workspace version not found'
}
$version = $Matches[1]
$sourceTag = ''
if ($RefName.StartsWith('v', [StringComparison]::Ordinal)) {
    $expectedTag = "v$version"
    if ($RefName -ne $expectedTag) {
        throw "tag $RefName does not match workspace version $version (expected $expectedTag)"
    }
    if ($RefName -eq 'v1.0.0') { throw 'v1.0.0 is immutable and cannot be republished' }
    $tagType = (& git -C $Root cat-file -t $RefName).Trim()
    if ($LASTEXITCODE -ne 0 -or $tagType -ne 'tag') {
        throw "release tag $RefName must be annotated"
    }
    $tagRevision = (& git -C $Root rev-list -n 1 $RefName).Trim()
    if ($LASTEXITCODE -ne 0 -or $tagRevision -ne $revision) {
        throw "release tag $RefName does not point at $revision"
    }
    $sourceTag = $RefName
}

$result = [ordered] @{
    revision = $revision
    sourceTag = $sourceTag
    version = $version
}
if ($env:GITHUB_OUTPUT) {
    "revision=$revision" | Add-Content -Path $env:GITHUB_OUTPUT -Encoding utf8NoBOM
    "source_tag=$sourceTag" | Add-Content -Path $env:GITHUB_OUTPUT -Encoding utf8NoBOM
    "version=$version" | Add-Content -Path $env:GITHUB_OUTPUT -Encoding utf8NoBOM
}
$result | ConvertTo-Json -Compress
