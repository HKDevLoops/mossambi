<#
.SYNOPSIS
    Mossambi Universal Windows Installer
    Universal File Conversion Platform, Indian Exam Studio & Archive Suite
#>
[CmdletBinding()]
param(
    [switch]$Unattended,
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\mossambi",
    [string]$Version = "2.1.0"
)

$ErrorActionPreference = "Stop"

# Enable UTF-8 console output
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# Certified Pantone Colors
$ESC = [char]27
$C_GREEN = "$ESC[38;2;120;190;32m"  # Pantone 376 C
$C_ZEST  = "$ESC[38;2;228;240;36m"  # Pantone 388 C
$C_GOLD  = "$ESC[38;2;255;184;28m"  # Pantone 1235 C
$C_CYAN  = "$ESC[38;2;60;200;230m"
$C_WHITE = "$ESC[38;2;245;245;247m"
$C_MUTED = "$ESC[38;2;160;160;165m"
$C_BOLD  = "$ESC[1m"
$C_RESET = "$ESC[0m"

function Show-Banner {
    $banner = "
$C_GREEN                       .-''''-.     _/\_
                     .'  ____  ``.  (____)
                    /  .' || ``.  \   ||
                   |  / \ || / \  |  ||
                   | ==  (())  == |  ||
                   |  \ / || \ /  |  ||
                    \  ``. || .'  /   ||
                     ``.  ``--'  .'
                       ``'-..-'``$C_RESET

$C_ZEST   ╭╮╭╮╭╮ ╭───╮ ╭────╮ ╭────╮ ╭───╮ ╭╮╭╮╭╮ ╭────╮ ╭─╮
   ││││││ │╭─╮│ │╭───╯ │╭───╯ │╭─╮│ ││││││ │╭─╮│ │ │
   ││││││ ││ ││ ╰───╮  ╰───╮  ││ ││ ││││││ │╰─╯│ │ │
   ││││││ │╰─╯│ ╭───╯  ╭───╯  │╰─╯│ ││││││ │╭─╮│ │ │
   ╰╯╰╯╰╯ ╰───╯ ╰────╯ ╰────╯ ╰─╰─╯ ╰╯╰╯╰╯ ╰───╯ ╰─╯$C_RESET

$C_GOLD   Universal File Conversion Platform · Indian Exam Form Studio$C_RESET
$C_MUTED   One Rust Core · Three Interfaces: CLI, TUI, and Floating Radial Widget$C_RESET
   $C_WHITE v$Version | 0.00% Zero-Loss Standard | Hardware Accelerated$C_RESET
"
    Write-Host $banner
}

function Detect-PackageManagers {
    $pms = @()
    if (Get-Command winget -ErrorAction SilentlyContinue) { $pms += "winget" }
    if (Get-Command scoop -ErrorAction SilentlyContinue)  { $pms += "scoop"  }
    if (Get-Command choco -ErrorAction SilentlyContinue)  { $pms += "choco"  }
    return $pms
}

function Install-DirectBinaries {
    Write-Host "`n$C_CYAN==>$C_RESET $C_BOLD Installing Mossambi to $InstallDir...$C_RESET"

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

    $scriptDir = if ($PSScriptRoot) { $PSScriptRoot } elseif ($MyInvocation.MyCommand.Path) { Split-Path -Parent $MyInvocation.MyCommand.Path } else { (Get-Location).Path }
    $localTarget = Join-Path $scriptDir "target\release"

    # 1. Check if built binaries exist locally
    if ((Test-Path (Join-Path $localTarget "mossambi.exe")) -and (Test-Path (Join-Path $localTarget "mossambi-gui.exe"))) {
        Write-Host "$C_GREEN✓ Found release build in $localTarget! Copying...$C_RESET"
        Copy-Item (Join-Path $localTarget "mossambi.exe") "$InstallDir\mossambi.exe" -Force
        Copy-Item (Join-Path $localTarget "mossambi-gui.exe") "$InstallDir\mossambi-gui.exe" -Force
        Copy-Item (Join-Path $localTarget "mossambi-tui.exe") "$InstallDir\mossambi-tui.exe" -Force
        if (Test-Path (Join-Path $localTarget "mossambi-widget.exe")) {
            Copy-Item (Join-Path $localTarget "mossambi-widget.exe") "$InstallDir\mossambi-widget.exe" -Force
        }
    } else {
        # 2. Download release archive from GitHub
        $zipUrl = "https://github.com/mossambi/mossambi/releases/download/v$Version/mossambi-v$Version-x86_64-pc-windows-msvc.zip"
        $tmpZip = Join-Path $env:TEMP "mossambi-v$Version.zip"
        Write-Host "$C_MUTED Downloading $zipUrl...$C_RESET"
        Invoke-WebRequest -Uri $zipUrl -OutFile $tmpZip -UseBasicParsing
        Expand-Archive -Path $tmpZip -DestinationPath $InstallDir -Force
        Remove-Item $tmpZip -Force -ErrorAction SilentlyContinue
    }

    Write-Host "$C_GREEN✓ Installed:$C_RESET $InstallDir\mossambi.exe"
    Write-Host "$C_GREEN✓ Installed:$C_RESET $InstallDir\mossambi-gui.exe"
    Write-Host "$C_GREEN✓ Installed:$C_RESET $InstallDir\mossambi-tui.exe"
    if (Test-Path "$InstallDir\mossambi-widget.exe") {
        Write-Host "$C_GREEN✓ Installed:$C_RESET $InstallDir\mossambi-widget.exe"
    }

    # Add to User PATH if not already present
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$InstallDir*") {
        Write-Host "$C_CYAN==>$C_RESET Adding $InstallDir to User PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$currentPath;$InstallDir", "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Host "$C_GREEN✓ User PATH updated successfully.$C_RESET"
    }

    # Create Windows Start Menu Shortcut
    $startMenuDir = [Environment]::GetFolderPath("Programs")
    $shortcutPath = Join-Path $startMenuDir "Mossambi.lnk"
    try {
        $wshShell = New-Object -ComObject WScript.Shell
        $shortcut = $wshShell.CreateShortcut($shortcutPath)
        $shortcut.TargetPath = "$InstallDir\mossambi-gui.exe"
        $shortcut.WorkingDirectory = $InstallDir
        $shortcut.Description = "Mossambi — Sweet Lime Transcoder & Indian Exam Studio"
        $shortcut.Save()
        Write-Host "$C_GREEN✓ Created Start Menu Shortcut:$C_RESET $shortcutPath"
    } catch {
        Write-Verbose "Could not create Start Menu shortcut: $_"
    }

    # Setup PowerShell Auto-Completions
    try {
        $compDir = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "WindowsPowerShell"
        if (-not (Test-Path $compDir)) {
            New-Item -ItemType Directory -Path $compDir -Force | Out-Null
        }
        $compFile = Join-Path $compDir "mossambi_completions.ps1"
        & "$InstallDir\mossambi.exe" completions powershell | Out-File -FilePath $compFile -Encoding utf8 -Force
        Write-Host "$C_GREEN✓ Generated PowerShell completions:$C_RESET $compFile"
    } catch {
        Write-Verbose "Completions generation note: $_"
    }
}

# --- Execution Entry Point ---
Show-Banner

$detectedPms = Detect-PackageManagers
Write-Host "$C_WHITE Architecture:$C_RESET x64 (AMD64) / Windows"
if ($detectedPms.Count -gt 0) {
    Write-Host "$C_WHITE Available Package Managers:$C_RESET $($detectedPms -join ', ')"
}

Write-Host "`n$C_BOLD Installation Methods:$C_RESET"
Write-Host "  $C_GREEN 1)$C_RESET Direct Standalone Install to $C_BOLD$InstallDir$C_RESET (Recommended)"
$optIndex = 2
if ($detectedPms -contains "winget") {
    Write-Host "  $C_GREEN $optIndex)$C_RESET Install via WinGet ($C_BOLD winget install Mossambi.Mossambi$C_RESET)"
    $optIndex++
}
if ($detectedPms -contains "scoop") {
    Write-Host "  $C_GREEN $optIndex)$C_RESET Install via Scoop ($C_BOLD scoop install mossambi$C_RESET)"
    $optIndex++
}

$choice = "1"
if (-not $Unattended -and [Environment]::UserInteractive) {
    $inputChoice = Read-Host "`nSelect an option [default: 1]"
    if ($inputChoice) { $choice = $inputChoice }
}

if ($choice -eq "1") {
    Install-DirectBinaries
} elseif ($choice -eq "2" -and ($detectedPms -contains "winget")) {
    Write-Host "$C_CYAN Invoking WinGet...$C_RESET"
    winget install --manifest "$PSScriptRoot\packaging\winget\Mossambi.Mossambi.yaml"
} elseif ($choice -eq "2" -or $choice -eq "3") {
    Install-DirectBinaries
} else {
    Install-DirectBinaries
}

$completionMsg = "
$C_GREEN======================================================================$C_RESET
$C_BOLD 🍋 Mossambi $Version installation completed successfully!$C_RESET

Commands ready to run from your terminal:
  $C_CYAN mossambi <file> [target]$C_RESET   - Fast UNIX-style conversion (``mv``-grammar)
  $C_CYAN mossambi-tui$C_RESET               - macOS-styled interactive terminal workspace
  $C_CYAN mossambi-gui$C_RESET               - Liquid Glass desktop studio & radial widget
  $C_CYAN mossambi --table-help$C_RESET      - Comprehensive command reference & examples

Global Activation:
  Press $C_BOLD WIN + CTRL$C_RESET or $C_BOLD drag cursor in 2 loops$C_RESET to summon the widget!
$C_GREEN======================================================================$C_RESET
"
Write-Host $completionMsg
