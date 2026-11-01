#!/usr/bin/env bash
# Quick rpm builder for Mossambi binary packages
set -euo pipefail

VERSION="2.1.0"
ARCH="$(uname -m)"
TOPDIR="$(pwd)/rpmbuild"

rm -rf "${TOPDIR}"
mkdir -p "${TOPDIR}/"{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# Prepare tarball
TARBALL="${TOPDIR}/SOURCES/mossambi-${VERSION}.tar.gz"
tar --exclude="./target" --exclude="./.git" -czf "${TARBALL}" -C . .

# Copy spec
cp -f packaging/rpm/mossambi.spec "${TOPDIR}/SPECS/"

rpmbuild --define "_topdir ${TOPDIR}" -ba "${TOPDIR}/SPECS/mossambi.spec"

echo "Successfully generated RPM in ${TOPDIR}/RPMS/${ARCH}/"
echo "Install with: sudo dnf install ${TOPDIR}/RPMS/${ARCH}/mossambi-${VERSION}-*.${ARCH}.rpm"
