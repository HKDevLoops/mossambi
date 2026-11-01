<#
.SYNOPSIS
    Builds the official Windows Installer (.msi) and distribution packages for Mossambi.
#>
[CmdletBinding()]
param(
    [string]$Version = "2.1.0",
    [string]$OutputDir = "$PSScriptRoot\..\..\target\dist"
)

$ErrorActionPreference = "Stop"

# Colors
$ESC = [char]27
$C_GREEN = "$ESC[38;2;120;190;32m"
$C_CYAN  = "$ESC[38;2;60;200;230m"
$C_GOLD  = "$ESC[38;2;255;184;28m"
$C_RESET = "$ESC[0m"

Write-Host "`n${C_GREEN}=======================================================${C_RESET}"
Write-Host "${C_GREEN}  MOSSAMBI WINDOWS INSTALLER BUILDER (.MSI & ZIP)${C_RESET}"
Write-Host "${C_GREEN}=======================================================${C_RESET}`n"

$ProjectRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
$ReleaseDir  = Join-Path $ProjectRoot "target\release"

# 1. Verify Binaries
$Binaries = @("mossambi.exe", "mossambi-gui.exe", "mossambi-widget.exe", "mossambi-tui.exe")
$Missing = @()

foreach ($bin in $Binaries) {
    $binPath = Join-Path $ReleaseDir $bin
    if (-not (Test-Path $binPath)) {
        $Missing += $bin
    }
}

if ($Missing.Count -gt 0) {
    Write-Host "${C_CYAN}==> Compiling release binaries...${C_RESET}"
    Push-Location $ProjectRoot
    cargo build --release
    Pop-Location
}

# 2. Ensure Output Directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$WxsPath = Join-Path $PSScriptRoot "mossambi.wxs"
$MsiOutput = Join-Path $OutputDir "mossambi-$Version-x64.msi"

# 3. Detect WiX Toolset
$Candle = Get-Command "candle" -ErrorAction SilentlyContinue
$Light  = Get-Command "light" -ErrorAction SilentlyContinue
$WixV4  = Get-Command "wix" -ErrorAction SilentlyContinue

# Also check standard WiX 3.11 / 3.14 paths
if (-not $Candle) {
    $Wix3Paths = @(
        "$env:WIX\bin\candle.exe",
        "${env:ProgramFiles(x86)}\WiX Toolset v3.11\bin\candle.exe",
        "${env:ProgramFiles(x86)}\WiX Toolset v3.14\bin\candle.exe",
        "$env:ProgramFiles\WiX Toolset v3.11\bin\candle.exe"
    )
    foreach ($p in $Wix3Paths) {
        if (Test-Path $p) {
            $Candle = $p
            $Light = (Join-Path (Split-Path $p) "light.exe")
            break
        }
    }
}

$MsiBuilt = $false

if ($WixV4) {
    Write-Host "${C_CYAN}==> Building MSI using WiX v4 (${WixV4.Source})...${C_RESET}"
    Push-Location $PSScriptRoot
    & wix build "mossambi.wxs" -o $MsiOutput
    Pop-Location
    if (Test-Path $MsiOutput) { $MsiBuilt = $true }
}
elseif ($Candle -and $Light) {
    Write-Host "${C_CYAN}==> Compiling MSI using WiX Toolset v3...${C_RESET}"
    $WixObj = Join-Path $OutputDir "mossambi.wixobj"
    Push-Location $PSScriptRoot
    & $Candle -arch x64 "mossambi.wxs" -out $WixObj
    & $Light -ext WixUIExtension $WixObj -out $MsiOutput
    Pop-Location
    if (Test-Path $MsiOutput) { $MsiBuilt = $true }
}

if ($MsiBuilt) {
    Write-Host "${C_GREEN}✓ Successfully generated MSI: $MsiOutput${C_RESET}"
} else {
    Write-Host "${C_GOLD}ℹ Note: WiX Toolset was not detected in PATH.${C_RESET}"
    Write-Host "  To compile the .msi directly, install WiX via:"
    Write-Host "    winget install WiX"
    Write-Host "    or: dotnet tool install --global wix"
    Write-Host "  Source WiX definition ready at: $WxsPath`n"
}

# 4. Also generate standard Portable Distribution ZIP bundle
$ZipOutput = Join-Path $OutputDir "mossambi-v$Version-windows-x64.zip"
$BundleStaging = Join-Path $OutputDir "mossambi-bundle-staging"

if (Test-Path $BundleStaging) { Remove-Item -Recurse -Force $BundleStaging }
New-Item -ItemType Directory -Path $BundleStaging -Force | Out-Null

# Copy binaries
foreach ($bin in $Binaries) {
    Copy-Item (Join-Path $ReleaseDir $bin) $BundleStaging
}

# Copy icons & documentation
$IconDir = Join-Path $BundleStaging "icons"
New-Item -ItemType Directory -Path $IconDir -Force | Out-Null
Copy-Item (Join-Path $ProjectRoot "assets\icons\mossambi.ico") $IconDir
Copy-Item (Join-Path $ProjectRoot "assets\icons\mossambi.svg") $IconDir
Copy-Item (Join-Path $ProjectRoot "assets\icons\mossambi-512x512.png") $IconDir
Copy-Item (Join-Path $ProjectRoot "README.md") $BundleStaging
Copy-Item (Join-Path $ProjectRoot "install.ps1") $BundleStaging

# Create quick runner launcher script inside zip
$SetupCmd = "@echo off`r`necho =======================================================`r`necho   Installing Mossambi Universal Converter & Studio`r`necho =======================================================`r`npowershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1`r`npause`r`n"
Set-Content -Path (Join-Path $BundleStaging "setup.bat") -Value $SetupCmd -Encoding ASCII

# Zip package
if (Test-Path $ZipOutput) { Remove-Item -Force $ZipOutput }
Compress-Archive -Path "$BundleStaging\*" -DestinationPath $ZipOutput -CompressionLevel Optimal
Remove-Item -Recurse -Force $BundleStaging

$binList = $Binaries -join ', '
Write-Host "${C_GREEN}✓ Successfully generated Portable Distribution Bundle: $ZipOutput${C_RESET}"
Write-Host "  Contains: $binList, install.ps1, setup.bat, and icons."
