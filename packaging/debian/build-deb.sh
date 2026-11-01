#!/usr/bin/env bash
# Quick deb builder for Mossambi binary packages
set -euo pipefail

VERSION="2.1.0"
ARCH="$(dpkg --print-architecture 2>/dev/null || echo amd64)"
PKG_DIR="mossambi_${VERSION}_${ARCH}"

rm -rf "${PKG_DIR}" "${PKG_DIR}.deb"
mkdir -p "${PKG_DIR}/DEBIAN"
mkdir -p "${PKG_DIR}/usr/bin"
mkdir -p "${PKG_DIR}/usr/share/applications"
mkdir -p "${PKG_DIR}/usr/share/bash-completion/completions"
mkdir -p "${PKG_DIR}/usr/share/zsh/vendor-completions"
mkdir -p "${PKG_DIR}/usr/share/fish/vendor_completions.d"

cat <<EOF > "${PKG_DIR}/DEBIAN/control"
Package: mossambi
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Depends: ffmpeg
Maintainer: Mossambi Systems Team <maintainers@mossambi.org>
Description: Universal lossless file converter and Indian exam tuner
 Zero-loss native file conversion, archive suite, and form optimizer.
EOF

# Copy binaries
cp -f target/release/mossambi "${PKG_DIR}/usr/bin/"
cp -f target/release/mossambi-gui "${PKG_DIR}/usr/bin/"
cp -f target/release/mossambi-tui "${PKG_DIR}/usr/bin/"
chmod 755 "${PKG_DIR}/usr/bin/"*

# Desktop file
cp -f packaging/assets/mossambi.desktop "${PKG_DIR}/usr/share/applications/"

# Generate completions
"${PKG_DIR}/usr/bin/mossambi" completions bash > "${PKG_DIR}/usr/share/bash-completion/completions/mossambi" || true
"${PKG_DIR}/usr/bin/mossambi" completions zsh > "${PKG_DIR}/usr/share/zsh/vendor-completions/_mossambi" || true
"${PKG_DIR}/usr/bin/mossambi" completions fish > "${PKG_DIR}/usr/share/fish/vendor_completions.d/mossambi.fish" || true

dpkg-deb --build --root-owner-group "${PKG_DIR}"
echo "Successfully generated: ${PKG_DIR}.deb"
echo "Install with: sudo apt install ./${PKG_DIR}.deb"
