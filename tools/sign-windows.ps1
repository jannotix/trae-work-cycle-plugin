#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Binary,
    [string] $TimestampUrl = 'http://timestamp.digicert.com',
    [string] $CertificateBase64Env = 'CYCLE_WINDOWS_CERTIFICATE_BASE64',
    [string] $CertificatePasswordEnv = 'CYCLE_WINDOWS_CERTIFICATE_PASSWORD',
    [string] $CertificateThumbprintEnv = 'CYCLE_WINDOWS_CERTIFICATE_THUMBPRINT'
)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Authenticode signing must run on Windows' }
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path

$certificateBase64 = [Environment]::GetEnvironmentVariable($CertificateBase64Env)
$certificatePassword = [Environment]::GetEnvironmentVariable($CertificatePasswordEnv)
$requestedThumbprint = [Environment]::GetEnvironmentVariable($CertificateThumbprintEnv)
if ([string]::IsNullOrWhiteSpace($requestedThumbprint) -and [string]::IsNullOrWhiteSpace($certificateBase64)) {
    throw "code-signing identity is missing: set $CertificateThumbprintEnv or $CertificateBase64Env"
}
if (-not [string]::IsNullOrWhiteSpace($certificateBase64) -and [string]::IsNullOrWhiteSpace($certificatePassword)) {
    throw "$CertificatePasswordEnv is required with $CertificateBase64Env"
}

$signTool = Get-Command signtool.exe -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $signTool) {
    $kits = Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'Windows Kits\10\bin'
    $candidate = Get-ChildItem -LiteralPath $kits -Filter signtool.exe -File -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.Directory.Name -eq 'x64' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if ($candidate) { $signTool = $candidate }
}
if (-not $signTool) { throw 'signtool.exe was not found' }
$signToolPath = if ($signTool.Source) { $signTool.Source } else { $signTool.FullName }

$codeSigningOid = '1.3.6.1.5.5.7.3.3'
$storePath = 'Cert:\CurrentUser\My'
$temporaryPfx = $null
$importedPaths = @()
$certificate = $null

try {
    if (-not [string]::IsNullOrWhiteSpace($requestedThumbprint)) {
        $normalized = $requestedThumbprint.Replace(' ', '').ToUpperInvariant()
        $matches = @(
            Get-ChildItem Cert:\CurrentUser\My, Cert:\LocalMachine\My -CodeSigningCert -ErrorAction SilentlyContinue |
                Where-Object { $_.Thumbprint -eq $normalized -and $_.HasPrivateKey }
        )
        if ($matches.Count -ne 1) {
            throw "expected exactly one code-signing certificate for thumbprint $normalized, found $($matches.Count)"
        }
        $certificate = $matches[0]
        if ($certificate.PSParentPath -like '*LocalMachine*') { $storePath = 'Cert:\LocalMachine\My' }
    } else {
        $before = @(Get-ChildItem Cert:\CurrentUser\My -ErrorAction SilentlyContinue | ForEach-Object Thumbprint)
        $temporaryPfx = Join-Path ([IO.Path]::GetTempPath()) ("trae-cycle-signing-{0}.pfx" -f [Guid]::NewGuid().ToString('N'))
        try {
            [IO.File]::WriteAllBytes($temporaryPfx, [Convert]::FromBase64String($certificateBase64))
        } catch {
            throw 'the code-signing certificate secret is not valid base64'
        }
        $securePassword = ConvertTo-SecureString $certificatePassword -AsPlainText -Force
        $imported = @(Import-PfxCertificate -FilePath $temporaryPfx -CertStoreLocation $storePath -Password $securePassword -Exportable:$false)
        $importedPaths = @($imported | Where-Object { $_.Thumbprint -notin $before } | ForEach-Object PSPath)
        $signers = @($imported | Where-Object {
                $_.HasPrivateKey -and
                ($_.EnhancedKeyUsageList | Where-Object { $_.ObjectId.Value -eq $codeSigningOid })
            })
        if ($signers.Count -ne 1) {
            throw "the PFX must contain exactly one private-key code-signing certificate, found $($signers.Count)"
        }
        $certificate = $signers[0]
    }

    $now = Get-Date
    if ($certificate.NotBefore -gt $now -or $certificate.NotAfter -le $now) {
        throw "the code-signing certificate is outside its validity period: $($certificate.NotBefore) to $($certificate.NotAfter)"
    }

    $arguments = @('sign', '/sha1', $certificate.Thumbprint, '/s', 'My')
    if ($storePath -like '*LocalMachine*') { $arguments += '/sm' }
    $arguments += @('/fd', 'SHA256', '/tr', $TimestampUrl, '/td', 'SHA256', $binaryPath)
    & $signToolPath @arguments
    if ($LASTEXITCODE -ne 0) { throw "signtool sign failed with exit code $LASTEXITCODE" }

    & $signToolPath verify /pa /all /v $binaryPath
    if ($LASTEXITCODE -ne 0) { throw "signtool verify failed with exit code $LASTEXITCODE" }
    $signature = Get-AuthenticodeSignature -FilePath $binaryPath
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Authenticode verification returned $($signature.Status)"
    }
    if (-not $signature.TimeStamperCertificate) {
        throw 'the Authenticode signature does not contain a trusted timestamp'
    }

    [pscustomobject]@{
        Binary = $binaryPath
        Sha256 = (Get-FileHash -LiteralPath $binaryPath -Algorithm SHA256).Hash
        SignerSubject = $signature.SignerCertificate.Subject
        SignerThumbprint = $signature.SignerCertificate.Thumbprint
        TimestampSubject = $signature.TimeStamperCertificate.Subject
    } | Format-List
} finally {
    foreach ($path in $importedPaths) {
        if ($path -and (Test-Path -LiteralPath $path)) { Remove-Item -LiteralPath $path -Force }
    }
    if ($temporaryPfx -and (Test-Path -LiteralPath $temporaryPfx)) {
        Remove-Item -LiteralPath $temporaryPfx -Force
    }
}
