[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "Programs\serial-logging-cli"),
    [switch]$NoPathUpdate
)

$ErrorActionPreference = "Stop"
$Repository = "DaqSense/serial-logging-cli"
$Target = "x86_64-pc-windows-msvc"

if (-not [Environment]::Is64BitOperatingSystem) {
    throw "serial-logging-cli currently requires 64-bit Windows."
}

if ($Version -eq "latest") {
    $LatestRelease = Invoke-RestMethod `
        -Uri "https://api.github.com/repos/$Repository/releases/latest" `
        -Headers @{ "User-Agent" = "serial-logging-cli-installer" }
    $Tag = $LatestRelease.tag_name
    if (-not $Tag) {
        throw "Could not determine the latest release version."
    }
    $DownloadBase = "https://github.com/$Repository/releases/latest/download"
} else {
    $Tag = if ($Version.StartsWith("v")) { $Version } else { "v$Version" }
    $DownloadBase = "https://github.com/$Repository/releases/download/$Tag"
}

$Asset = "serial-logging-cli-$Tag-$Target.zip"
$TemporaryDir = Join-Path ([System.IO.Path]::GetTempPath()) ("serial-logging-cli-" + [guid]::NewGuid())
$Archive = Join-Path $TemporaryDir $Asset
$Checksums = Join-Path $TemporaryDir "SHA256SUMS"

try {
    New-Item -ItemType Directory -Path $TemporaryDir -Force | Out-Null

    Write-Host "Downloading serial-logging-cli ($Target)..."
    Invoke-WebRequest -Uri "$DownloadBase/$Asset" -OutFile $Archive -UseBasicParsing
    Invoke-WebRequest -Uri "$DownloadBase/SHA256SUMS" -OutFile $Checksums -UseBasicParsing

    $ChecksumLine = Get-Content $Checksums | Where-Object { $_ -match "\s+$([regex]::Escape($Asset))$" }
    if (-not $ChecksumLine) {
        throw "$Asset is missing from SHA256SUMS."
    }

    $ExpectedChecksum = ($ChecksumLine -split "\s+")[0].ToLowerInvariant()
    $ActualChecksum = (Get-FileHash -Path $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($ActualChecksum -ne $ExpectedChecksum) {
        throw "Checksum verification failed for $Asset."
    }

    Expand-Archive -Path $Archive -DestinationPath $TemporaryDir -Force
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item `
        -Path (Join-Path $TemporaryDir "serial-logging-cli.exe") `
        -Destination (Join-Path $InstallDir "serial-logging-cli.exe") `
        -Force

    if (-not $NoPathUpdate) {
        $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $PathEntries = @($UserPath -split ";" | Where-Object { $_ })
        if ($PathEntries -notcontains $InstallDir) {
            $UpdatedPath = (@($PathEntries) + $InstallDir) -join ";"
            [Environment]::SetEnvironmentVariable("Path", $UpdatedPath, "User")
            Write-Host "Added $InstallDir to your user PATH."
        }
        if (($env:Path -split ";") -notcontains $InstallDir) {
            $env:Path = "$InstallDir;$env:Path"
        }
    }

    Write-Host "Installed serial-logging-cli to $InstallDir\serial-logging-cli.exe"
    Write-Host "Open a new terminal, then try: serial-logging-cli --list"
} finally {
    if (Test-Path $TemporaryDir) {
        Remove-Item -Path $TemporaryDir -Recurse -Force
    }
}
