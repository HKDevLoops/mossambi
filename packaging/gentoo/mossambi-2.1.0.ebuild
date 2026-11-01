# Copyright 2026 Gentoo Authors
# Distributed under the terms of the GNU General Public License v2

EAPI=8

CRATES="
"

inherit cargo desktop xdg

DESCRIPTION="Zero-loss native desktop file converter, archive suite, and Indian exam studio"
HOMEPAGE="https://github.com/mossambi/mossambi"
SRC_URI="https://github.com/mossambi/mossambi/archive/v${PV}.tar.gz -> ${P}.tar.gz"

LICENSE="MIT Apache-2.0"
SLOT="0"
KEYWORDS="~amd64 ~arm64"
IUSE="+gui +tui"

RDEPEND="
	media-video/ffmpeg
	media-libs/alsa-lib
	media-libs/mesa
	x11-libs/libxkbcommon
	dev-libs/wayland
"
DEPEND="${RDEPEND}"
BDEPEND="
	virtual/pkgconfig
	>=virtual/rust-1.75.0
"

src_compile() {
	cargo_src_compile
}

src_install() {
	dobin "$(cargo_target_dir)/mossambi"
	use gui && dobin "$(cargo_target_dir)/mossambi-gui"
	use gui && dobin "$(cargo_target_dir)/mossambi-widget"
	use tui && dobin "$(cargo_target_dir)/mossambi-tui"

	if use gui; then
		domenu packaging/assets/mossambi.desktop
	fi

	# Generate shell completions
	"$(cargo_target_dir)/mossambi" completions bash > mossambi.bash || die
	"$(cargo_target_dir)/mossambi" completions zsh > _mossambi || die
	"$(cargo_target_dir)/mossambi" completions fish > mossambi.fish || die

	newbashcomp mossambi.bash mossambi
	newzshcomp _mossambi _mossambi
	dofishcomp mossambi.fish
}
