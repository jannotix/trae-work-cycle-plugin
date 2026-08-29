#requires -Version 7.0
[CmdletBinding()]
param(
    [string] $Root = (Split-Path $PSScriptRoot -Parent),
    [string] $Dist = ''
)

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path $Root).Path
$failures = [Collections.Generic.List[string]]::new()

function Assert-Contract([bool] $Condition, [string] $Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}

$mcp = Get-Content (Join-Path $Root 'plugin/install/mcp.example.json') -Raw | ConvertFrom-Json
$command = [string] $mcp.mcpServers.'trae-cycle'.command
Assert-Contract (-not [string]::IsNullOrWhiteSpace($command)) 'MCP command is missing'
Assert-Contract ($command -notmatch '\s') "MCP command path contains whitespace: $command"

$readme = Get-Content (Join-Path $Root 'README.md') -Raw
$manual = Get-Content (Join-Path $Root 'documentation/USER_MANUAL.md') -Raw
Assert-Contract ($readme -notmatch '%LOCALAPPDATA%\\Trae Cycle\\bin') 'README uses a Windows executable path with spaces'
Assert-Contract ($manual -notmatch '%LOCALAPPDATA%\\Trae Cycle\\bin') 'User manual uses a Windows executable path with spaces'
Assert-Contract ($readme -match 'Windows x64') 'README does not name the Windows v1 lane'
Assert-Contract ($readme -match 'WSL') 'README does not name the WSL v1 lane'
Assert-Contract ($readme -match 'compatible but untested') 'README does not label macOS compatible but untested'

if ($Dist) {
    $distPath = (Resolve-Path $Dist).Path
    $version = [regex]::Match((Get-Content (Join-Path $Root 'production/Cargo.toml') -Raw), '(?s)\[workspace\.package\].*?version\s*=\s*"([^"]+)"').Groups[1].Value
    $skillArchive = Join-Path $distPath "cycle-delivery-skill-$version.zip"
    $pluginArchive = Join-Path $distPath "trae-work-cycle-plugin-$version.zip"
    Assert-Contract (Test-Path $skillArchive) "skill archive is missing: $skillArchive"
    Assert-Contract (Test-Path $pluginArchive) "plugin archive is missing: $pluginArchive"
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path $skillArchive) {
        $zip = [System.IO.Compression.ZipFile]::OpenRead($skillArchive)
        try {
            $entries = @($zip.Entries | ForEach-Object FullName)
            Assert-Contract ($entries -contains 'SKILL.md') 'skill archive does not contain root-level SKILL.md'
            Assert-Contract ($entries -contains 'references/tools.md') 'skill archive does not contain root-level references/tools.md'
            Assert-Contract (-not ($entries | Where-Object { $_ -like 'cycle-delivery/*' })) 'skill archive has an extra cycle-delivery directory'
            Assert-Contract (-not ($zip.Entries | Where-Object { $_.LastWriteTime.DateTime -ne [DateTime]::new(1980, 1, 1, 0, 0, 0) })) 'skill archive timestamps are not deterministic'
        } finally {
            $zip.Dispose()
        }
    }
    if (Test-Path $pluginArchive) {
        $zip = [System.IO.Compression.ZipFile]::OpenRead($pluginArchive)
        try {
            Assert-Contract (-not ($zip.Entries | Where-Object { $_.LastWriteTime.DateTime -ne [DateTime]::new(1980, 1, 1, 0, 0, 0) })) 'plugin archive timestamps are not deterministic'
        } finally {
            $zip.Dispose()
        }
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "package contract failed: $($failures.Count) violation(s)"
}
Write-Host 'package contract passed'
