#requires -Version 7.0
[CmdletBinding(DefaultParameterSetName = 'Package')]
param(
    [string] $Root = (Split-Path $PSScriptRoot -Parent),
    [Parameter(ParameterSetName = 'Package')]
    [string[]] $Assets = @(),
    [Parameter(ParameterSetName = 'Package')]
    [string] $Output = '',
    [Parameter(Mandatory = $true, ParameterSetName = 'Verify')]
    [string] $Verify
)

$ErrorActionPreference = 'Stop'

$Product = 'trae-work-cycle-plugin'
$WorkspaceLicense = 'FSL-1.1-MIT'

$AllowedLicenses = @(
    'MIT', 'Apache-2.0', 'ISC', 'BSD-2-Clause', 'BSD-3-Clause', '0BSD',
    'Zlib', 'Unlicense', 'CC0-1.0', 'CDLA-Permissive-2.0', 'BSL-1.0',
    'Unicode-3.0', 'Unicode-DFS-2016', 'BlueOak-1.0.0'
)
$AllowedExceptions = @('LLVM-exception')

$SkillAllowlist = @(
    'plugin/skill/cycle-delivery/SKILL.md',
    'plugin/skill/cycle-delivery/references/tools.md',
    'plugin/skill/cycle-delivery/references/evidence-protocol.md'
)
$PluginAllowlist = @(
    'README.md',
    'LICENSE',
    'NOTICE',
    'plugin/command/cycle.md',
    'plugin/install/mcp.example.json'
) + $SkillAllowlist

function Split-Expression([string] $Text, [string] $Operator) {
    $depth = 0
    $parts = @()
    $current = New-Object System.Text.StringBuilder
    for ($i = 0; $i -lt $Text.Length; $i++) {
        $char = $Text[$i]
        if ($char -eq '(') { $depth++ }
        if ($char -eq ')') { $depth-- }
        if ($depth -eq 0 -and $i -le $Text.Length - $Operator.Length) {
            $window = $Text.Substring($i, $Operator.Length)
            if ($window -eq $Operator) {
                $parts += $current.ToString().Trim()
                $current.Clear() | Out-Null
                $i += $Operator.Length - 1
                continue
            }
        }
        [void] $current.Append($char)
    }
    $parts += $current.ToString().Trim()
    return $parts
}

function Test-LicenseExpression([string] $Expression) {
    if (-not $Expression) { return $false }
    $normalized = $Expression -replace '/', ' OR '
    foreach ($choice in (Split-Expression $normalized ' OR ')) {
        $terms = @(Split-Expression $choice ' AND ')
        $all = $true
        foreach ($term in $terms) {
            $clean = $term.Trim('()').Trim()
            if ($AllowedLicenses -contains $clean) { continue }
            if ($clean -match '^(.+?)\s+WITH\s+(.+)$') {
                $base = $Matches[1].Trim('()').Trim()
                $exception = $Matches[2]
                if ($AllowedLicenses -contains $base -and $AllowedExceptions -contains $exception) { continue }
            }
            if (($clean -match ' OR ' -or $clean -match '\(') -and (Test-LicenseExpression $clean)) { continue }
            $all = $false
            break
        }
        if ($all) { return $true }
    }
    return $false
}

function Get-Sha256([string] $Path) {
    (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Copy-Allowlist([string[]] $Allowlist, [string] $SourceRoot, [string] $Stage) {
    foreach ($relative in $Allowlist) {
        $source = Join-Path $SourceRoot ($relative -replace '/', [IO.Path]::DirectorySeparatorChar)
        if (-not (Test-Path $source)) { throw "allowlisted file missing: $relative" }
        if ($relative.StartsWith('plugin/skill/')) {
            $destination = Join-Path $Stage ($relative.Substring('plugin/skill/'.Length) -replace '/', [IO.Path]::DirectorySeparatorChar)
        } elseif ($relative.StartsWith('plugin/')) {
            $destination = Join-Path $Stage ($relative.Substring('plugin/'.Length) -replace '/', [IO.Path]::DirectorySeparatorChar)
        } else {
            $destination = Join-Path $Stage ($relative -replace '/', [IO.Path]::DirectorySeparatorChar)
        }
        $directory = Split-Path $destination -Parent
        if (-not (Test-Path $directory)) { New-Item -ItemType Directory -Path $directory | Out-Null }
        Copy-Item $source $destination
    }
}

function New-ZipFromStage([string] $Stage, [string] $ZipPath) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path $ZipPath) { Remove-Item $ZipPath }
    [System.IO.Compression.ZipFile]::CreateFromDirectory($Stage, $ZipPath, [System.IO.Compression.CompressionLevel]::Optimal, $false)
}

function Get-CargoMetadata([string] $ProductionRoot) {
    $raw = & cargo metadata --format-version 1 --manifest-path (Join-Path $ProductionRoot 'Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $raw | ConvertFrom-Json -AsHashtable
}

function Test-LicenseGate($Metadata) {
    $violations = @()
    $workspaceIds = @($Metadata.workspace_members)
    $externals = @()
    foreach ($package in $Metadata.packages) {
        if ($workspaceIds -contains $package.id) {
            if ($package.license -ne $WorkspaceLicense) {
                $violations += "$($package.name): workspace license is '$($package.license)', expected '$WorkspaceLicense'"
            }
        } else {
            $externals += $package
            if (-not (Test-LicenseExpression $package.license)) {
                $violations += "$($package.name) $($package.version): license '$($package.license)' is not on the allowlist"
            }
        }
    }
    if ($violations.Count -gt 0) {
        $violations | ForEach-Object { Write-Error $_ }
        throw "license gate failed: $($violations.Count) violation(s)"
    }
    return $externals
}

function New-Sbom($Metadata, $Externals, [string] $Version, [string] $Revision, [string] $Path) {
    $components = @()
    $seen = @{}
    foreach ($package in (@($Metadata.packages) | Sort-Object name, version)) {
        if ($seen.ContainsKey($package.id)) { continue }
        $seen[$package.id] = $true
        $component = [ordered] @{
            'type'    = 'library'
            'bom-ref' = "pkg:cargo/$($package.name)@$($package.version)"
            'name'    = $package.name
            'version' = $package.version
            'purl'    = "pkg:cargo/$($package.name)@$($package.version)"
        }
        $expression = $null
        if ($package.license) {
            $expression = ($package.license -replace '/', ' OR ')
            $component['licenses'] = @(@{ 'expression' = $expression })
        } else {
            $component['licenses'] = @(@{ 'license' = @{ 'name' = 'NOASSERTION' } })
        }
        $components += $component
    }
    $bom = [ordered] @{
        '$schema'      = 'http://cyclonedx.org/schema/bom-1.5.schema.json'
        'bomFormat'    = 'CycloneDX'
        'specVersion'  = '1.5'
        'serialNumber' = "urn:uuid:$([Guid]::NewGuid().ToString())"
        'version'      = 1
        'metadata'     = [ordered] @{
            'timestamp' = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
            'component' = [ordered] @{
                'type'    = 'application'
                'name'    = $Product
                'version' = $Version
                'licenses' = @(@{ 'expression' = $WorkspaceLicense })
                'purl'    = "pkg:github/jannotix/$Product@$Version"
            }
            'properties' = @(
                @{ 'name' = 'sourceRevision'; 'value' = $Revision }
            )
        }
        'components'   = $components
    }
    $bom | ConvertTo-Json -Depth 10 | Set-Content -Path $Path -Encoding utf8NoBOM
}

function New-ThirdPartyNotices($Externals, [string] $Path, [string] $Version) {
    $lines = @(
        '# Third-Party Notices',
        '',
        "$Product $Version includes the following third-party Rust crates.",
        'Each crate carries its own license as declared in its manifest;',
        'license texts are available in the crate sources and on crates.io.',
        ''
    )
    $groups = $Externals | Group-Object { if ($_.license) { $_.license } else { '(no license declared)' } } | Sort-Object Name
    foreach ($group in $groups) {
        $lines += "## $($group.Name)"
        ''
        $lines += (($group.Group | Sort-Object name | ForEach-Object { "- $($_.name) $($_.version)" }) -join "`n")
        $lines += ''
    }
    $lines -join "`n" | Set-Content -Path $Path -Encoding utf8NoBOM
}

function Read-WorkspaceVersion([string] $ProductionRoot) {
    $cargo = Get-Content (Join-Path $ProductionRoot 'Cargo.toml') -Raw
    if ($cargo -notmatch '(?s)\[workspace\.package\].*?version\s*=\s*"([^"]+)"') { throw 'workspace version not found' }
    $Matches[1]
}

function New-Manifest([object[]] $Artifacts, [string] $Version, [string] $Revision, [string] $Tag, [string] $Path) {
    $manifest = [ordered] @{
        'schemaVersion'  = 1
        'product'        = $Product
        'version'        = $Version
        'createdUtc'     = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
        'sourceRevision' = $Revision
        'sourceTag'      = $Tag
        'artifacts'      = @($Artifacts | Sort-Object name | ForEach-Object {
            [ordered] @{ 'name' = $_.name; 'sha256' = $_.sha256; 'bytes' = $_.bytes }
        })
    }
    $manifest | ConvertTo-Json -Depth 6 | Set-Content -Path $Path -Encoding utf8NoBOM
}

if ($PSCmdlet.ParameterSetName -eq 'Verify') {
    $manifestPath = Join-Path $Verify 'MANIFEST.json'
    if (-not (Test-Path $manifestPath)) { throw "MANIFEST.json not found in $Verify" }
    $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json -AsHashtable
    $head = (& git -C $Root rev-parse HEAD).Trim()
    if ($manifest['sourceRevision'] -ne $head) {
        throw "revision mismatch: manifest $($manifest['sourceRevision']) vs source $head"
    }
    $failures = @()
    foreach ($artifact in $manifest['artifacts']) {
        $path = Join-Path $Verify $artifact['name']
        if (-not (Test-Path $path)) { $failures += "missing artifact $($artifact['name'])"; continue }
        $digest = Get-Sha256 $path
        if ($digest -ne $artifact['sha256']) { $failures += "digest mismatch $($artifact['name'])" }
    }
    if ($failures.Count -gt 0) { $failures | ForEach-Object { Write-Error $_ }; throw 'verification failed' }
    Write-Host "verified $($manifest['artifacts'].Count) artifacts at revision $head"
    exit 0
}

$Root = (Resolve-Path $Root).Path
$ProductionRoot = Join-Path $Root 'production'
if (-not $Output) { $Output = Join-Path $Root 'dist' }
$dist = (New-Item -ItemType Directory -Force -Path $Output).FullName

$version = Read-WorkspaceVersion $ProductionRoot
$revision = (& git -C $Root rev-parse HEAD 2>$null)
if (-not $revision -or $LASTEXITCODE -ne 0) { throw 'packaging requires a git repository (no revision)' }
$revision = $revision.Trim()
$tag = (& git -C $Root describe --exact-match --tags HEAD 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $tag) { $tag = $null } else { $tag = $tag.Trim() }

Write-Host "packaging $Product $version at $revision$(if ($tag) { " (tag $tag)" })"

$metadata = Get-CargoMetadata $ProductionRoot
$externals = Test-LicenseGate $metadata
Write-Host "license gate: 9 workspace crates + $($externals.Count) dependencies allowed"

$staging = Join-Path $dist '.staging'
if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }

$skillStage = Join-Path $staging 'skill'
New-Item -ItemType Directory -Path $skillStage | Out-Null
Copy-Allowlist -Allowlist $SkillAllowlist -SourceRoot $Root -Stage $skillStage
$skillZip = Join-Path $dist "cycle-delivery-skill-$version.zip"
New-ZipFromStage $skillStage $skillZip

$pluginStage = Join-Path $staging 'plugin'
New-Item -ItemType Directory -Path $pluginStage | Out-Null
Copy-Allowlist -Allowlist $PluginAllowlist -SourceRoot $Root -Stage $pluginStage
$pluginZip = Join-Path $dist "$Product-$version.zip"
New-ZipFromStage $pluginStage $pluginZip

Remove-Item $staging -Recurse -Force

$sbomPath = Join-Path $dist 'SBOM.cdx.json'
New-Sbom $metadata $externals $version $revision $sbomPath

$noticesPath = Join-Path $dist 'THIRD-PARTY-NOTICES.md'
New-ThirdPartyNotices $externals $noticesPath $version

$artifactNames = @(
    "cycle-delivery-skill-$version.zip",
    "$Product-$version.zip",
    'SBOM.cdx.json',
    'THIRD-PARTY-NOTICES.md'
)
foreach ($asset in $Assets) {
    if (-not (Test-Path $asset)) { throw "asset missing: $asset" }
    $target = Join-Path $dist (Split-Path $asset -Leaf)
    Copy-Item $asset $target -Force
    $artifactNames += (Split-Path $asset -Leaf)
}

$artifacts = foreach ($name in $artifactNames) {
    $path = Join-Path $dist $name
    [pscustomobject] @{ name = $name; sha256 = (Get-Sha256 $path); bytes = (Get-Item $path).Length }
}
$manifestPath = Join-Path $dist 'MANIFEST.json'
New-Manifest $artifacts $version $revision $tag $manifestPath

$sums = ($artifacts | Sort-Object name | ForEach-Object { "$($_.sha256)  $($_.name)" }) -join "`n"
[System.IO.File]::WriteAllText((Join-Path $dist 'SHA256SUMS.txt'), $sums + "`n")

Write-Host "artifacts in $dist"
$artifacts | Sort-Object name | Format-Table name, bytes -AutoSize
