Name:           mossambi
Version:        2.1.0
Release:        0
Summary:        Universal zero-loss file converter and Indian exam studio
License:        MIT OR Apache-2.0
Group:          Productivity/File utilities
URL:            https://github.com/mossambi/mossambi
Source0:        %{name}-%{version}.tar.gz
BuildRequires:  cargo
BuildRequires:  rust >= 1.75
BuildRequires:  alsa-devel
BuildRequires:  Mesa-libGL-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  wayland-devel
BuildRequires:  pkg-config
Requires:       ffmpeg

%description
Mossambi is a high-performance native desktop file conversion suite.
Features CLI, Ratatui TUI, and egui Liquid Glass Desktop Studio & Radial Widget.

%prep
%setup -q

%build
cargo build --release --workspace

%install
install -Dm755 target/release/mossambi %{buildroot}%{_bindir}/mossambi
install -Dm755 target/release/mossambi-gui %{buildroot}%{_bindir}/mossambi-gui
install -Dm755 target/release/mossambi-widget %{buildroot}%{_bindir}/mossambi-widget
install -Dm755 target/release/mossambi-tui %{buildroot}%{_bindir}/mossambi-tui
install -Dm644 packaging/assets/mossambi.desktop %{buildroot}%{_datadir}/applications/mossambi.desktop

%files
%license LICENSE*
%doc README.md
%{_bindir}/mossambi
%{_bindir}/mossambi-gui
%{_bindir}/mossambi-widget
%{_bindir}/mossambi-tui
%{_datadir}/applications/mossambi.desktop

%changelog
* Sun Nov 01 2026 Mossambi Systems Team <maintainers@mossambi.org>
- Initial openSUSE package for version 2.1.0
