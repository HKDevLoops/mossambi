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
    $localTarget = if (Test-Path (Join-Path $scriptDir "target\release\mossambi.exe")) {
        Join-Path $scriptDir "target\release"
    } elseif (Test-Path "target\release\mossambi.exe") {
        (Resolve-Path "target\release").Path
    } else {
        $null
    }

    # 1. Check if built binaries exist locally
    if ($localTarget -and (Test-Path (Join-Path $localTarget "mossambi.exe"))) {
        Write-Host "$C_GREEN✓ Found release build in $localTarget! Copying...$C_RESET"
        Copy-Item (Join-Path $localTarget "mossambi.exe") "$InstallDir\mossambi.exe" -Force
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

    # Copy Icon Assets
    $installIconDir = Join-Path $InstallDir "icons"
    if (-not (Test-Path $installIconDir)) {
        New-Item -ItemType Directory -Path $installIconDir -Force | Out-Null
    }
    $localIcons = Join-Path $PSScriptRoot "assets\icons"
    if (Test-Path $localIcons) {
        Copy-Item "$localIcons\*" $installIconDir -Recurse -Force
    }

    # Add to User PATH if not already present
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$InstallDir*") {
        Write-Host "$C_CYAN==>$C_RESET Adding $InstallDir to User PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$currentPath;$InstallDir", "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Host "$C_GREEN✓ User PATH updated successfully.$C_RESET"
    }

    # Create Windows Start Menu & Desktop Shortcuts with Custom Icon
    $startMenuDir = [Environment]::GetFolderPath("Programs")
    $desktopDir = [Environment]::GetFolderPath("Desktop")
    $icoFile = Join-Path $installIconDir "mossambi.ico"

    try {
        $wshShell = New-Object -ComObject WScript.Shell

        # 1. Start Menu Studio Shortcut
        $shortcutPath = Join-Path $startMenuDir "Mossambi Studio.lnk"
        $shortcut = $wshShell.CreateShortcut($shortcutPath)
        $shortcut.TargetPath = "$InstallDir\mossambi.exe"
        $shortcut.Arguments = "--app"
        $shortcut.WorkingDirectory = $InstallDir
        $shortcut.Description = "Mossambi — Universal File Explorer & Conversion Studio"
        if (Test-Path $icoFile) { $shortcut.IconLocation = "$icoFile,0" }
        $shortcut.Save()
        Write-Host "$C_GREEN✓ Created Start Menu Shortcut:$C_RESET $shortcutPath"

        # 2. Start Menu Radial Widget Shortcut
        $widgetShortcutPath = Join-Path $startMenuDir "Mossambi Radial Widget.lnk"
        $wShortcut = $wshShell.CreateShortcut($widgetShortcutPath)
        $wShortcut.TargetPath = "$InstallDir\mossambi.exe"
        $wShortcut.Arguments = "--widget"
        $wShortcut.WorkingDirectory = $InstallDir
        $wShortcut.Description = "Summon Floating Sweet Lime Radial Widget"
        if (Test-Path $icoFile) { $wShortcut.IconLocation = "$icoFile,0" }
        $wShortcut.Save()
        Write-Host "$C_GREEN✓ Created Start Menu Widget Shortcut:$C_RESET $widgetShortcutPath"

        # 3. Desktop Shortcut
        $desktopShortcutPath = Join-Path $desktopDir "Mossambi.lnk"
        $dShortcut = $wshShell.CreateShortcut($desktopShortcutPath)
        $dShortcut.TargetPath = "$InstallDir\mossambi.exe"
        $dShortcut.Arguments = "--app"
        $dShortcut.WorkingDirectory = $InstallDir
        $dShortcut.Description = "Mossambi — Universal File Explorer & Conversion Studio"
        if (Test-Path $icoFile) { $dShortcut.IconLocation = "$icoFile,0" }
        $dShortcut.Save()
        Write-Host "$C_GREEN✓ Created Desktop Shortcut:$C_RESET $desktopShortcutPath"
    } catch {
        Write-Verbose "Could not create shortcuts: $_"
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
} elseif (($choice -eq "3" -or ($choice -eq "2" -and -not ($detectedPms -contains "winget"))) -and ($detectedPms -contains "scoop")) {
    Write-Host "$C_CYAN Invoking Scoop...$C_RESET"
    scoop install "$PSScriptRoot\packaging\scoop\mossambi.json"
} else {
    Install-DirectBinaries
}

# Ask user whether they want Mossambi to run in the background (like Whisper Flow)
if (-not $Unattended -and [Environment]::UserInteractive) {
    Write-Host "`n$C_BOLD Background Resident Service (Whisper Flow Style):$C_RESET"
    Write-Host "  Mossambi can run quietly in the background to listen for global hotkeys:"
    Write-Host "    $C_GREEN Win+Shift+C$C_RESET ➔ Summon Floating Radial Wheel under cursor"
    Write-Host "    $C_GREEN Win+Ctrl+C$C_RESET  ➔ Open Full Desktop Studio GUI"
    $enableBg = Read-Host "`nEnable background resident service on startup? [Y/n] (default: Y)"
    if (-not $enableBg -or $enableBg -match "^[yY]") {
        try {
            $runKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
            Set-ItemProperty -Path $runKey -Name "Mossambi" -Value "`"$InstallDir\mossambi.exe`" daemon run" -Force
            Write-Host "$C_GREEN✓ Enabled Mossambi Background Daemon on Windows Startup!$C_RESET"

            # Start daemon now
            Start-Process -FilePath "$InstallDir\mossambi.exe" -ArgumentList "daemon", "run" -WindowStyle Hidden
            Write-Host "$C_GREEN✓ Mossambi daemon started in background.$C_RESET"
        } catch {
            Write-Verbose "Could not register startup: $_"
        }
    } else {
        Write-Host "$C_MUTED Skipped background startup. You can enable it anytime via 'mossambi daemon install'.$C_RESET"
    }
}

$completionMsg = "
$C_GREEN======================================================================$C_RESET
$C_BOLD 🍋 Mossambi $Version installation completed successfully!$C_RESET

Commands ready to run from your terminal:
  $C_CYAN mossambi <file> [target]$C_RESET   - Fast UNIX-style conversion (``mv``-grammar)
  $C_CYAN mossambi-widget$C_RESET            - Topbarless floating radial wheel widget
  $C_CYAN mossambi-gui$C_RESET               - Desktop studio & exam optimizer
  $C_CYAN mossambi-tui$C_RESET               - macOS-styled interactive terminal workspace
  $C_CYAN mossambi daemon$C_RESET            - Manage background resident service
  $C_CYAN mossambi --table-help$C_RESET      - Comprehensive command reference & examples

Global Hotkeys (Whisper Flow Style):
  Press $C_BOLD Win + Shift + C$C_RESET to summon the Sweet Lime Radial Wheel under cursor!
  Press $C_BOLD Win + Ctrl + C$C_RESET  to open Desktop Studio GUI!
$C_GREEN======================================================================$C_RESET
"
Write-Host $completionMsg
