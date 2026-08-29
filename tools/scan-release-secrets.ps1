#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Path
)

$ErrorActionPreference = 'Stop'
$Path = (Resolve-Path $Path).Path
$textExtensions = @('.json', '.md', '.txt', '.yaml', '.yml', '.toml')
$patterns = [ordered] @{
    'OpenAI-style API key' = '\bsk-[A-Za-z0-9_-]{16,}'
    'GitHub token' = '\b(?:ghp|github_pat)_[A-Za-z0-9_]{20,}'
    'private key' = '-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'
    'bearer token' = '\bBearer\s+[A-Za-z0-9._-]{20,}'
    'credential assignment' = '\b(?:API_KEY|AUTH_TOKEN|ACCESS_TOKEN|SECRET_KEY)\s*=\s*[^\s<]+'
    'personal Windows path' = 'C:\\Users\\(?!YOU(?:\\|$))[^\\\s]+'
    'personal Unix home' = '/home/(?!YOU(?:/|$))[^/\s]+'
}
$findings = [Collections.Generic.List[string]]::new()

function Test-Text([string] $Name, [string] $Text) {
    foreach ($pattern in $patterns.GetEnumerator()) {
        if ($Text -match $pattern.Value) {
            $script:findings.Add("$Name contains $($pattern.Key)")
        }
    }
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
foreach ($file in (Get-ChildItem -LiteralPath $Path -File | Sort-Object Name)) {
    if ($file.Extension -eq '.zip') {
        $archive = [System.IO.Compression.ZipFile]::OpenRead($file.FullName)
        try {
            foreach ($entry in $archive.Entries) {
                if ($entry.Length -gt 4 * 1024 * 1024 -or [IO.Path]::GetExtension($entry.FullName) -notin $textExtensions) {
                    continue
                }
                $reader = [IO.StreamReader]::new($entry.Open(), [Text.Encoding]::UTF8, $true)
                try {
                    Test-Text "$($file.Name)!/$($entry.FullName)" $reader.ReadToEnd()
                } finally {
                    $reader.Dispose()
                }
            }
        } finally {
            $archive.Dispose()
        }
        continue
    }
    if ($file.Length -le 4 * 1024 * 1024 -and $file.Extension -in $textExtensions) {
        Test-Text $file.Name ([IO.File]::ReadAllText($file.FullName))
    }
}

if ($findings.Count -gt 0) {
    $findings | ForEach-Object { Write-Error $_ }
    throw "release secret scan failed: $($findings.Count) finding(s)"
}
Write-Host 'release secret scan passed'
