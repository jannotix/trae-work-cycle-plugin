#requires -Version 7.0
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$captured = [Collections.Generic.List[string]]::new()
& cargo test --workspace --all-features --locked --color never 2>&1 | ForEach-Object {
    $line = $_.ToString()
    Write-Host $line
    $captured.Add($line)
}
$exitCode = $LASTEXITCODE
if ($exitCode -eq 0) { exit 0 }

$start = [Math]::Max(0, $captured.Count - 80)
$tail = ($captured.GetRange($start, $captured.Count - $start) -join "`n")
$tail = $tail -replace '\bsk-[A-Za-z0-9_-]{16,}', '[REDACTED]'
$tail = $tail -replace '[A-Za-z]:\\Users\\[^\\\s]+', '[WINDOWS_USER_PATH]'
$tail = $tail -replace '/home/(?:runner|workflowci)/[^\s:]+', '[LINUX_USER_PATH]'
if ($tail.Length -gt 6000) { $tail = $tail.Substring($tail.Length - 6000) }
$annotation = $tail.Replace('%', '%25').Replace("`r", '%0D').Replace("`n", '%0A')
Write-Output "::error title=Workspace tests failed (exit $exitCode)::$annotation"
exit $exitCode
