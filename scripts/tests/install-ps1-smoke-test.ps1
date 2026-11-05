$ErrorActionPreference = 'Stop'

$installDir = Join-Path $env:RUNNER_TEMP 'openstream-installer-smoke'
$lockedInstallDir = Join-Path $env:RUNNER_TEMP 'openstream-installer-locked-smoke'
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$processPath = $env:Path
$previousOnboard = $env:OPENSTREAM_ONBOARD
$previousVariant = $env:OPENSTREAM_VARIANT
$legacyLock = $null

try {
    $env:OPENSTREAM_ONBOARD = 'never'
    $env:OPENSTREAM_VARIANT = 'slim'
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    $legacyInstalled = Join-Path $installDir 'openstream.exe'
    Set-Content -LiteralPath $legacyInstalled -Value 'legacy executable'
    $output = & (Join-Path $PSScriptRoot '..\install.ps1') -Version '1.4.0' -InstallDir $installDir -NoTls 6>&1 | Out-String

    $installed = Join-Path $installDir 'ost.exe'
    if (-not (Test-Path -LiteralPath $installed -PathType Leaf)) {
        throw 'The Windows installer did not place ost.exe in the requested directory.'
    }
    if ($output -notmatch 'Installed ost \(slim,') {
        throw 'OPENSTREAM_VARIANT=slim did not select the slim Windows asset.'
    }
    if (Test-Path -LiteralPath $legacyInstalled) {
        throw 'The Windows installer did not migrate openstream.exe to ost.exe.'
    }

    $help = & $installed --help 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0 -or $help -notmatch 'USAGE:') {
        throw 'The installed Windows executable did not pass its help smoke test.'
    }

    New-Item -ItemType Directory -Force -Path $lockedInstallDir | Out-Null
    $lockedLegacy = Join-Path $lockedInstallDir 'openstream.exe'
    Set-Content -LiteralPath $lockedLegacy -Value 'locked legacy executable'
    $legacyLock = [System.IO.File]::Open(
        $lockedLegacy,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::Read
    )
    $lockedOutput = & (Join-Path $PSScriptRoot '..\install.ps1') -Version '1.4.0' -InstallDir $lockedInstallDir -NoTls 6>&1 | Out-String
    if ($lockedOutput -notmatch 'Could not remove legacy openstream\.exe; it may still be in use') {
        throw 'The Windows installer did not warn when legacy executable cleanup failed.'
    }
    if (-not (Test-Path -LiteralPath (Join-Path $lockedInstallDir 'ost.exe') -PathType Leaf)) {
        throw 'Legacy cleanup failure prevented ost.exe from being installed.'
    }
}
finally {
    if ($null -ne $legacyLock) { $legacyLock.Dispose() }
    [Environment]::SetEnvironmentVariable('Path', $userPath, 'User')
    $env:Path = $processPath
    if ($null -eq $previousOnboard) { Remove-Item Env:OPENSTREAM_ONBOARD -ErrorAction SilentlyContinue }
    else { $env:OPENSTREAM_ONBOARD = $previousOnboard }
    if ($null -eq $previousVariant) { Remove-Item Env:OPENSTREAM_VARIANT -ErrorAction SilentlyContinue }
    else { $env:OPENSTREAM_VARIANT = $previousVariant }
    Remove-Item -LiteralPath $installDir -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $lockedInstallDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host 'Windows installer smoke test OK'
