# 📦 Mossambi Package Manager & Distribution Directory

Mossambi provides official, turnkey package manager manifests and automated installer methods for all major operating systems and Linux distributions.

---

## 🪟 Windows Package Managers

### 1. Windows Package Manager (`winget`)
```powershell
winget install Mossambi.Mossambi
# Or from local manifest:
winget install --manifest packaging/winget/Mossambi.Mossambi.yaml
```

### 2. Scoop
```powershell
# Add custom bucket or install directly:
scoop install packaging/scoop/mossambi.json
```

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
