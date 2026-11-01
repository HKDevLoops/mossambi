# 📦 Mossambi Package Manager & Distribution Directory

Mossambi provides official, turnkey binary installers, package manager manifests, and automated distribution pipelines for all major operating systems and Linux distributions.

---

## 🪟 Windows Binary Installers & Package Managers

### 1. Windows Installer (`.msi`) & Portable Bundle (`.zip`)
```powershell
# Build both the official .msi and portable .zip bundle:
pwsh -File packaging/windows/build-msi.ps1

# The output is generated in target/dist/:
#  - target/dist/mossambi-2.1.0-x64.msi
#  - target/dist/mossambi-v2.1.0-windows-x64.zip
```
- **WiX Toolset Manifest**: [`packaging/windows/mossambi.wxs`](windows/mossambi.wxs) defines the 64-bit MSI package with custom desktop/start-menu shortcuts, `PATH` registration, and ARP uninstaller metadata.
- **Portable ZIP Bundle**: Contains all 4 binaries (`mossambi.exe`, `mossambi-gui.exe`, `mossambi-widget.exe`, `mossambi-tui.exe`), `setup.bat`, `install.ps1`, and official Sweet Lime icon assets.

### 2. Windows Package Manager (`winget`)
```powershell
winget install Mossambi.Mossambi
# Or from local manifest:
winget install --manifest packaging/winget/Mossambi.Mossambi.yaml
```

### 3. Scoop
```powershell
# Add custom bucket or install directly:
scoop install packaging/scoop/mossambi.json
```

---

## 🎨 Official Sweet Lime Brand Icons (`assets/icons/`)

Mossambi includes custom-crafted, publication-grade Sweet Lime fruit icon artwork across all standard dimensions:

| Format / Dimension | Path | Usage |
|---|---|---|
| **Master Scalable Vector** | [`assets/icons/mossambi.svg`](../assets/icons/mossambi.svg) | High-DPI displays, Linux scalable vector themes, web banners |
| **Windows Multi-Res Icon** | [`assets/icons/mossambi.ico`](../assets/icons/mossambi.ico) | Windows Explorer, Taskbar, MSI Installer, Start Menu (16 to 256 px) |
| **Ultra-High Resolution** | [`assets/icons/mossambi-1024x1024.png`](../assets/icons/mossambi-1024x1024.png) | macOS App Store, publication marketing, high-res splashes |
| **Standard App Icon** | [`assets/icons/mossambi-512x512.png`](../assets/icons/mossambi-512x512.png) | Linux `/usr/share/pixmaps/`, Flathub, Snap Store |
| **Desktop Icon** | [`assets/icons/mossambi-256x256.png`](../assets/icons/mossambi-256x256.png) | Embedded in `mossambi-gui` and `mossambi-widget` window frames |
| **System Tray Icon** | [`assets/icons/mossambi-128x128.png`](../assets/icons/mossambi-128x128.png) | App launchers, dock icons |
| **Medium Shell Icon** | [`assets/icons/mossambi-64x64.png`](../assets/icons/mossambi-64x64.png) | Linux hicolor standard |
| **Small Shell Icon** | [`assets/icons/mossambi-48x48.png`](../assets/icons/mossambi-48x48.png) | Windows Control Panel & notification area |
| **Menu / Tray Icon** | [`assets/icons/mossambi-32x32.png`](../assets/icons/mossambi-32x32.png) | Taskbar and window titlebar |
| **Smallest Micro Icon** | [`assets/icons/mossambi-16x16.png`](../assets/icons/mossambi-16x16.png) | Windows System Tray notification area, tab icons |

---

## 🐧 Linux Distributions

### 1. Arch Linux / Manjaro (`pacman` & `yay`)
```bash
# Via AUR helper:
yay -S mossambi-bin
# Or build locally:
cd packaging/arch && makepkg -si
```

### 2. Debian / Ubuntu / Linux Mint / Pop!_OS (`apt`)
```bash
# Build .deb package locally:
./packaging/debian/build-deb.sh
# Install with apt:
sudo apt install ./mossambi_2.1.0_amd64.deb
```

### 3. Fedora / RHEL / CentOS / AlmaLinux (`dnf` & `rpm`)
```bash
# Build RPM package:
rpmbuild -ba packaging/rpm/mossambi.spec
# Install with dnf:
sudo dnf install ~/rpmbuild/RPMS/x86_64/mossambi-2.1.0-1*.rpm
```

### 4. openSUSE Tumbleweed & Leap (`zypper`)
```bash
# Build via rpmbuild:
rpmbuild -ba packaging/zypper/mossambi-opensuse.spec
# Install with zypper:
sudo zypper install ~/rpmbuild/RPMS/x86_64/mossambi-2.1.0-*.rpm
```

### 5. Gentoo Linux (`emerge` & `portage`)
```bash
# Copy ebuild into your local overlay:
mkdir -p /usr/local/portage/app-misc/mossambi
cp packaging/gentoo/mossambi-2.1.0.ebuild /usr/local/portage/app-misc/mossambi/
ebuild /usr/local/portage/app-misc/mossambi/mossambi-2.1.0.ebuild manifest
emerge --ask app-misc/mossambi
```

### 6. Nix / NixOS (`nix` flakes & profiles)
```bash
# Run immediately without installing:
nix run github:mossambi/mossambi

# Install permanently to current user profile:
nix profile install github:mossambi/mossambi

# Or in NixOS configuration.nix via flake input:
# environment.systemPackages = [ inputs.mossambi.packages.${system}.default ];
```

### 7. GNU Guix
```bash
guix package -f packaging/guix/mossambi.scm
```

### 8. Alpine Linux (`apk`)
```bash
cd packaging/alpine && abuild -r
apk add ~/packages/alpine/x86_64/mossambi-2.1.0-r0.apk
```

---

## 🍏 macOS & Multi-Platform (`brew`)

### Homebrew (macOS & Linux)
```bash
brew install packaging/homebrew/mossambi.rb
```

---

## ⚡ Universal One-Line Quick Installers

### Linux & macOS (POSIX `bash` / `zsh` / `sh`):
```bash
curl -fsSL https://raw.githubusercontent.com/mossambi/mossambi/main/install.sh | bash
```

### Windows (PowerShell):
```powershell
irm https://raw.githubusercontent.com/mossambi/mossambi/main/install.ps1 | iex
```
