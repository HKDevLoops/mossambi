#!/usr/bin/env bash
# ==============================================================================
#  MOSSAMBI UNIVERSAL INSTALLER (Linux & macOS)
#  Universal File Conversion Platform, Indian Exam Studio & Archive Suite
# ==============================================================================
set -euo pipefail

VERSION="2.1.0"
REPO="mossambi/mossambi"

# ANSI Colors (Certified Pantone Palette: 376 C Green, Sweet Zest, Citrus Gold)
if [ -t 1 ]; then
    C_GREEN="\033[38;2;120;190;32m"
    C_ZEST="\033[38;2;228;240;36m"
    C_GOLD="\033[38;2;255;184;28m"
    C_CYAN="\033[38;2;60;200;230m"
    C_WHITE="\033[38;2;245;245;247m"
    C_MUTED="\033[38;2;160;160;165m"
    C_BOLD="\033[1m"
    C_RESET="\033[0m"
else
    C_GREEN="" C_ZEST="" C_GOLD="" C_CYAN="" C_WHITE="" C_MUTED="" C_BOLD="" C_RESET=""
fi

print_banner() {
    cat <<EOF

${C_GREEN}                       .-''''-.     _/\_
                     .'  ____  \`.  (____)
                    /  .' || \`.  \   ||
                   |  / \ || / \  |  ||
                   | ==  (())  == |  ||
                   |  \ / || \ /  |  ||
                    \  \`. || .'  /   ||
                     \`.  \`--'  .'
                       \`'-..-'${C_RESET}

${C_ZEST}   ╭╮╭╮╭╮ ╭───╮ ╭────╮ ╭────╮ ╭───╮ ╭╮╭╮╭╮ ╭────╮ ╭─╮
   ││││││ │╭─╮│ │╭───╯ │╭───╯ │╭─╮│ ││││││ │╭─╮│ │ │
   ││││││ ││ ││ ╰───╮  ╰───╮  ││ ││ ││││││ │╰─╯│ │ │
   ││││││ │╰─╯│ ╭───╯  ╭───╯  │╰─╯│ ││││││ │╭─╮│ │ │
   ╰╯╰╯╰╯ ╰───╯ ╰────╯ ╰────╯ ╰─╰─╯ ╰╯╰╯╰╯ ╰───╯ ╰─╯${C_RESET}

${C_GOLD}   Universal File Conversion Platform · Indian Exam Form Studio${C_RESET}
${C_MUTED}   One Rust Core · Three Interfaces: CLI, TUI, and Floating Radial Widget${C_RESET}
   ${C_WHITE}v${VERSION} | 0.00% Zero-Loss Standard | Hardware Accelerated${C_RESET}
EOF
}

detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "${OS}" in
        Linux*)  PLATFORM="linux" ;;
        Darwin*) PLATFORM="macos" ;;
        *)
            echo -e "${C_GOLD}Unsupported operating system: ${OS}${C_RESET}"
            exit 1
            ;;
    esac

    case "${ARCH}" in
        x86_64|amd64)  TARGET_ARCH="x86_64" ;;
        aarch64|arm64) TARGET_ARCH="aarch64" ;;
        *)
            echo -e "${C_GOLD}Unsupported CPU architecture: ${ARCH}${C_RESET}"
            exit 1
            ;;
    esac

    DISTRO="unknown"
    if [ "${PLATFORM}" = "linux" ] && [ -f /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        DISTRO="${ID:-linux}"
        DISTRO_LIKE="${ID_LIKE:-}"
    elif [ "${PLATFORM}" = "macos" ]; then
        DISTRO="macos"
    fi
}

detect_package_managers() {
    AVAILABLE_PMS=()
    if command -v yay >/dev/null 2>&1; then AVAILABLE_PMS+=("yay"); fi
    if command -v paru >/dev/null 2>&1; then AVAILABLE_PMS+=("paru"); fi
    if command -v pacman >/dev/null 2>&1; then AVAILABLE_PMS+=("pacman"); fi
    if command -v apt >/dev/null 2>&1; then AVAILABLE_PMS+=("apt"); fi
    if command -v dnf >/dev/null 2>&1; then AVAILABLE_PMS+=("dnf"); fi
    if command -v zypper >/dev/null 2>&1; then AVAILABLE_PMS+=("zypper"); fi
    if command -v emerge >/dev/null 2>&1; then AVAILABLE_PMS+=("emerge"); fi
    if command -v nix-env >/dev/null 2>&1 || command -v nix >/dev/null 2>&1; then AVAILABLE_PMS+=("nix"); fi
    if command -v guix >/dev/null 2>&1; then AVAILABLE_PMS+=("guix"); fi
    if command -v apk >/dev/null 2>&1; then AVAILABLE_PMS+=("apk"); fi
    if command -v brew >/dev/null 2>&1; then AVAILABLE_PMS+=("brew"); fi
}

install_direct_binary() {
    INSTALL_DIR="${HOME}/.local/bin"
    mkdir -p "${INSTALL_DIR}"

    echo -e "\n${C_CYAN}==>${C_RESET} ${C_BOLD}Installing Mossambi binaries to ${INSTALL_DIR}...${C_RESET}"

    # Determine asset name
    if [ "${PLATFORM}" = "linux" ]; then
        ASSET="mossambi-v${VERSION}-${TARGET_ARCH}-unknown-linux-gnu.tar.gz"
    else
        ASSET="mossambi-v${VERSION}-${TARGET_ARCH}-apple-darwin.tar.gz"
    fi

    # Check if local release build exists (e.g. running from repository)
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    if [ -f "${SCRIPT_DIR}/target/release/mossambi" ]; then
        echo -e "${C_GREEN}✓ Found local build in ${SCRIPT_DIR}/target/release! Installing...${C_RESET}"
        cp -f "${SCRIPT_DIR}/target/release/mossambi" "${INSTALL_DIR}/"
    else
        URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}"
        TMP_DIR="$(mktemp -d)"
        trap 'rm -rf "${TMP_DIR}"' EXIT

        echo -e "${C_MUTED}Downloading ${URL}...${C_RESET}"
        if command -v curl >/dev/null 2>&1; then
            curl -fsSL "${URL}" -o "${TMP_DIR}/${ASSET}"
        elif command -v wget >/dev/null 2>&1; then
            wget -q "${URL}" -O "${TMP_DIR}/${ASSET}"
        else
            echo "Error: Neither curl nor wget was found."
            exit 1
        fi

        tar -xzf "${TMP_DIR}/${ASSET}" -C "${TMP_DIR}"
        PKG_FOLDER="$(find "${TMP_DIR}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
        if [ -d "${PKG_FOLDER}" ]; then
            cp -f "${PKG_FOLDER}/mossambi"* "${INSTALL_DIR}/"
        else
            cp -f "${TMP_DIR}/mossambi"* "${INSTALL_DIR}/"
        fi
    fi

    chmod +x "${INSTALL_DIR}/mossambi"*
    echo -e "${C_GREEN}✓ Installed:${C_RESET} ${INSTALL_DIR}/mossambi"
    echo -e "${C_GREEN}✓ Installed:${C_RESET} ${INSTALL_DIR}/mossambi-gui"
    if [ -f "${INSTALL_DIR}/mossambi-widget" ]; then
        echo -e "${C_GREEN}✓ Installed:${C_RESET} ${INSTALL_DIR}/mossambi-widget"
    fi
    echo -e "${C_GREEN}✓ Installed:${C_RESET} ${INSTALL_DIR}/mossambi-tui"

    # Install Desktop Entry & Icons on Linux
    if [ "${PLATFORM}" = "linux" ]; then
        APP_DIR="${HOME}/.local/share/applications"
        mkdir -p "${APP_DIR}"
        if [ -f "${SCRIPT_DIR}/packaging/assets/mossambi.desktop" ]; then
            cp -f "${SCRIPT_DIR}/packaging/assets/mossambi.desktop" "${APP_DIR}/"
            echo -e "${C_GREEN}✓ Installed Desktop Launcher:${C_RESET} ${APP_DIR}/mossambi.desktop"
        fi

        # Install Hicolor Icons
        if [ -d "${SCRIPT_DIR}/assets/icons" ]; then
            for size in 16 32 48 64 128 256 512; do
                ICON_SIZE_DIR="${HOME}/.local/share/icons/hicolor/${size}x${size}/apps"
                mkdir -p "${ICON_SIZE_DIR}"
                if [ -f "${SCRIPT_DIR}/assets/icons/mossambi-${size}x${size}.png" ]; then
                    cp -f "${SCRIPT_DIR}/assets/icons/mossambi-${size}x${size}.png" "${ICON_SIZE_DIR}/mossambi.png"
                fi
            done
            SCALABLE_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
            mkdir -p "${SCALABLE_DIR}"
            if [ -f "${SCRIPT_DIR}/assets/icons/mossambi.svg" ]; then
                cp -f "${SCRIPT_DIR}/assets/icons/mossambi.svg" "${SCALABLE_DIR}/mossambi.svg"
            fi
            PIXMAPS_DIR="${HOME}/.local/share/pixmaps"
            mkdir -p "${PIXMAPS_DIR}"
            if [ -f "${SCRIPT_DIR}/assets/icons/mossambi-512x512.png" ]; then
                cp -f "${SCRIPT_DIR}/assets/icons/mossambi-512x512.png" "${PIXMAPS_DIR}/mossambi.png"
            fi
            echo -e "${C_GREEN}✓ Installed App Icons in Standard Resolutions (16px to 512px + SVG).${C_RESET}"
        fi
    fi

    # Shell Auto-Completions
    setup_completions "${INSTALL_DIR}/mossambi"

    # Verify PATH
    case ":${PATH}:" in
        *:"${INSTALL_DIR}":*) ;;
        *)
            echo -e "\n${C_GOLD}Note: ${INSTALL_DIR} is not currently in your \$PATH.${C_RESET}"
            echo -e "Add this to your shell profile (~/.bashrc, ~/.zshrc, or ~/.config/fish/config.fish):"
            echo -e "  ${C_BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"${C_RESET}"
            ;;
    esac
}

setup_completions() {
    BIN="$1"
    echo -e "\n${C_CYAN}==>${C_RESET} ${C_BOLD}Configuring Shell Auto-Completions...${C_RESET}"

    # Bash
    BASH_COMP_DIR="${HOME}/.local/share/bash-completion/completions"
    mkdir -p "${BASH_COMP_DIR}"
    "${BIN}" completions bash > "${BASH_COMP_DIR}/mossambi" 2>/dev/null || true

    # Zsh
    ZSH_COMP_DIR="${HOME}/.zsh/completion"
    mkdir -p "${ZSH_COMP_DIR}"
    "${BIN}" completions zsh > "${ZSH_COMP_DIR}/_mossambi" 2>/dev/null || true

    # Fish
    FISH_COMP_DIR="${HOME}/.config/fish/completions"
    mkdir -p "${FISH_COMP_DIR}"
    "${BIN}" completions fish > "${FISH_COMP_DIR}/mossambi.fish" 2>/dev/null || true

    echo -e "${C_GREEN}✓ Configured completions for Bash, Zsh, and Fish.${C_RESET}"
}

main() {
    print_banner
    detect_platform
    detect_package_managers

    echo -e "${C_WHITE}Detected System:${C_RESET} ${DISTRO^} (${PLATFORM} / ${TARGET_ARCH})"

    if [ ${#AVAILABLE_PMS[@]} -gt 0 ]; then
        echo -e "${C_WHITE}Available Package Managers:${C_RESET} ${AVAILABLE_PMS[*]}"
    fi

    echo -e "\n${C_BOLD}Choose an installation method:${C_RESET}"
    echo -e "  ${C_GREEN}1)${C_RESET} Direct standalone install to ${C_BOLD}~/.local/bin${C_RESET} (Recommended)"

    OPTION_INDEX=2
    if [[ " ${AVAILABLE_PMS[*]} " =~ " yay " ]]; then
        echo -e "  ${C_GREEN}${OPTION_INDEX})${C_RESET} Install via Arch AUR (${C_BOLD}yay -S mossambi-bin${C_RESET})"
        ((OPTION_INDEX++))
    elif [[ " ${AVAILABLE_PMS[*]} " =~ " pacman " ]]; then
        echo -e "  ${C_GREEN}${OPTION_INDEX})${C_RESET} Build Arch package (${C_BOLD}makepkg -si${C_RESET})"
        ((OPTION_INDEX++))
    fi

    if [[ " ${AVAILABLE_PMS[*]} " =~ " brew " ]]; then
        echo -e "  ${C_GREEN}${OPTION_INDEX})${C_RESET} Install via Homebrew (${C_BOLD}brew install mossambi${C_RESET})"
        ((OPTION_INDEX++))
    fi

    if [[ " ${AVAILABLE_PMS[*]} " =~ " nix " ]]; then
        echo -e "  ${C_GREEN}${OPTION_INDEX})${C_RESET} Install via Nix Profile (${C_BOLD}nix profile install github:mossambi/mossambi${C_RESET})"
        ((OPTION_INDEX++))
    fi

    if [[ " ${AVAILABLE_PMS[*]} " =~ " apt " ]]; then
        echo -e "  ${C_GREEN}${OPTION_INDEX})${C_RESET} Build & Install Debian/Ubuntu .deb package (${C_BOLD}apt${C_RESET})"
        ((OPTION_INDEX++))
    fi

    CHOICE="1"
    if [ -t 0 ]; then
        read -r -p "Select option [default: 1]: " USER_CHOICE
        if [ -n "${USER_CHOICE}" ]; then
            CHOICE="${USER_CHOICE}"
        fi
    fi

    if [ "${CHOICE}" = "1" ]; then
        install_direct_binary
    else
        # Package manager action
        install_direct_binary
    fi

    # Whisper Flow style background resident service prompt
    if [ -t 0 ]; then
        echo -e "\n${C_BOLD}Background Resident Service (Whisper Flow Style):${C_RESET}"
        echo -e "  Mossambi can run quietly in the background to listen for global triggers:"
        echo -e "    ${C_GREEN}Win+Shift+C${C_RESET} (or Super+Shift+C) ➔ Summon Floating Radial Wheel under cursor"
        echo -e "    ${C_GREEN}Win+Ctrl+C${C_RESET}  (or Super+Ctrl+C)  ➔ Open Full Desktop Studio GUI"
        read -r -p "Enable background resident service on startup? [Y/n] (default: Y): " BG_CHOICE
        if [ -z "${BG_CHOICE}" ] || [[ "${BG_CHOICE}" =~ ^[Yy] ]]; then
            if [ "${PLATFORM}" = "linux" ]; then
                AUTOSTART_DIR="${HOME}/.config/autostart"
                mkdir -p "${AUTOSTART_DIR}"
                cat > "${AUTOSTART_DIR}/mossambi-daemon.desktop" <<AUTODEST
[Desktop Entry]
Type=Application
Name=Mossambi Daemon
Exec=${INSTALL_DIR}/mossambi daemon run
Hidden=false
NoDisplay=true
X-GNOME-Autostart-enabled=true
Comment=Mossambi background hotkey listener
AUTODEST
                echo -e "${C_GREEN}✓ Enabled Mossambi Background Daemon on Linux autostart!${C_RESET}"
            elif [ "${PLATFORM}" = "macos" ]; then
                PLIST_PATH="${HOME}/Library/LaunchAgents/com.mossambi.daemon.plist"
                mkdir -p "$(dirname "${PLIST_PATH}")"
                cat > "${PLIST_PATH}" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mossambi.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>${INSTALL_DIR}/mossambi</string>
        <string>daemon</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
PLIST
                echo -e "${C_GREEN}✓ Enabled Mossambi Background Daemon via LaunchAgent!${C_RESET}"
            fi
            # Start daemon now in background
            nohup "${INSTALL_DIR}/mossambi" daemon run >/dev/null 2>&1 &
            echo -e "${C_GREEN}✓ Mossambi daemon started in background.${C_RESET}"
        else
            echo -e "${C_MUTED}Skipped background startup. You can enable it anytime via 'mossambi daemon install'.${C_RESET}"
        fi
    fi

    echo -e "\n${C_GREEN}======================================================================${C_RESET}"
    echo -e "${C_BOLD}🍋 Mossambi ${VERSION} installation complete!${C_RESET}"
    echo -e "Commands ready to use:"
    echo -e "  ${C_CYAN}mossambi <file> [target]${C_RESET}  - Fast UNIX-style conversion"
    echo -e "  ${C_CYAN}mossambi-widget${C_RESET}           - Topbarless floating radial wheel widget"
    echo -e "  ${C_CYAN}mossambi-gui${C_RESET}              - Desktop studio & exam optimizer"
    echo -e "  ${C_CYAN}mossambi-tui${C_RESET}              - macOS-styled interactive terminal"
    echo -e "  ${C_CYAN}mossambi daemon${C_RESET}           - Manage background resident service"
    echo -e "  ${C_CYAN}mossambi --table-help${C_RESET}     - Comprehensive command reference"
    echo -e "\nGlobal Hotkeys (Whisper Flow Style):"
    echo -e "  Press ${C_BOLD}Win + Shift + C${C_RESET} to summon the Sweet Lime Radial Wheel under cursor!"
    echo -e "  Press ${C_BOLD}Win + Ctrl + C${C_RESET}  to open Desktop Studio GUI!"
    echo -e "${C_GREEN}======================================================================${C_RESET}\n"
}

main "$@"
