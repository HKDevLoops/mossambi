Name:           mossambi
Version:        2.1.0
Release:        1%{?dist}
Summary:        Universal zero-loss native file converter and Indian exam studio
License:        MIT or ASL 2.0
URL:            https://github.com/mossambi/mossambi
Source0:        https://github.com/mossambi/mossambi/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.75
BuildRequires:  alsa-lib-devel
BuildRequires:  mesa-libGL-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  wayland-devel
BuildRequires:  pkgconfig

Requires:       ffmpeg
Recommends:     mesa-va-drivers
Recommends:     libva-intel-driver

%description
Mossambi is a high-performance native desktop file conversion suite powered by
a shared pure-Rust conversion engine with three user interfaces:
 - mossambi (UNIX-style CLI with stream-copy and mv grammar)
 - mossambi-tui (macOS-styled interactive terminal workspace)
 - mossambi-gui (Desktop studio with floating 8-carpel radial widget)
Features 0.00% Zero-Loss mathematical remuxing, 100% scorecard Indian exam
document optimization (UPSC, SSC, IBPS), and streaming quality inspection.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --workspace

%install
rm -rf %{buildroot}
install -Dm755 target/release/mossambi %{buildroot}%{_bindir}/mossambi
install -Dm755 target/release/mossambi-gui %{buildroot}%{_bindir}/mossambi-gui
install -Dm755 target/release/mossambi-widget %{buildroot}%{_bindir}/mossambi-widget
install -Dm755 target/release/mossambi-tui %{buildroot}%{_bindir}/mossambi-tui

install -Dm644 packaging/assets/mossambi.desktop %{buildroot}%{_datadir}/applications/mossambi.desktop

# Shell completions
install -d %{buildroot}%{_datadir}/bash-completion/completions
%{buildroot}%{_bindir}/mossambi completions bash > %{buildroot}%{_datadir}/bash-completion/completions/mossambi || :

install -d %{buildroot}%{_datadir}/zsh/site-functions
%{buildroot}%{_bindir}/mossambi completions zsh > %{buildroot}%{_datadir}/zsh/site-functions/_mossambi || :

install -d %{buildroot}%{_datadir}/fish/vendor_completions.d
%{buildroot}%{_bindir}/mossambi completions fish > %{buildroot}%{_datadir}/fish/vendor_completions.d/mossambi.fish || :

%files
%license LICENSE*
%doc README.md
%{_bindir}/mossambi
%{_bindir}/mossambi-gui
%{_bindir}/mossambi-widget
%{_bindir}/mossambi-tui
%{_datadir}/applications/mossambi.desktop
%{_datadir}/bash-completion/completions/mossambi
%{_datadir}/zsh/site-functions/_mossambi
%{_datadir}/fish/vendor_completions.d/mossambi.fish

%changelog
* Sun Nov 01 2026 Mossambi Systems Team <maintainers@mossambi.org> - 2.1.0-1
- Release version 2.1.0 with CLI, TUI, GUI, and Radial Widget interfaces
