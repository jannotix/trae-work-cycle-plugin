#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Binary,
    [string] $ExpectedVersion = ''
)

$ErrorActionPreference = 'Stop'
$binaryPath = (Resolve-Path $Binary).Path
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$dataDirectory = Join-Path $tempRoot ("trae-cycle-smoke-{0}" -f [Guid]::NewGuid().ToString('N'))
$dataDirectory = [IO.Path]::GetFullPath($dataDirectory)
if (-not $dataDirectory.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'smoke data directory escaped the system temporary directory'
}
New-Item -ItemType Directory -Path $dataDirectory | Out-Null

try {
    $request = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"release-smoke","version":"1.0.0"}}}'
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $binaryPath
    foreach ($argument in @('mcp', '--data-dir', $dataDirectory)) {
        [void] $startInfo.ArgumentList.Add($argument)
    }
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void] $process.Start()
    $outputTask = $process.StandardOutput.ReadToEndAsync()
    $errorTask = $process.StandardError.ReadToEndAsync()
    $process.StandardInput.WriteLine($request)
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(60000)) {
        $process.Kill($true)
        throw 'extracted runtime smoke test timed out'
    }
    if ($process.ExitCode -ne 0) {
        throw "extracted runtime smoke test failed: $($errorTask.Result)"
    }
    $lines = @($outputTask.Result -split "`r?`n" | Where-Object { $_.Trim() })
    if ($lines.Count -ne 1) { throw "expected one MCP response, received $($lines.Count)" }
    $response = $lines[0] | ConvertFrom-Json
    if ($response.result.serverInfo.name -ne 'trae-cycle') {
        throw "unexpected server identity: $($response.result.serverInfo.name)"
    }
    if ($ExpectedVersion -and $response.result.serverInfo.version -ne $ExpectedVersion) {
        throw "runtime version $($response.result.serverInfo.version) does not match $ExpectedVersion"
    }
    Write-Host "runtime smoke passed: $($response.result.serverInfo.name) $($response.result.serverInfo.version)"
} finally {
    if ((Test-Path -LiteralPath $dataDirectory) -and $dataDirectory.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $dataDirectory -Recurse -Force
    }
}
