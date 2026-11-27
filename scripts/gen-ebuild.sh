#!/usr/bin/env bash
# Generate a Gentoo ebuild for the current version.
#
# Tauri has no Gentoo bundler, and Gentoo installs from source anyway
# (source-based distro), so instead of a binary we ship the package recipe
# itself: users drop the file into a local overlay and `emerge` it. The
# ebuild uses Gentoo's cargo eclass and pulls every crate from Cargo.lock
# into SRC_URI via CARGO_CRATE_URIS.
#
# Usage: scripts/gen-ebuild.sh [output dir]
set -euo pipefail

OUT="${1:-dist-ebuild}"

# Pick a working Python — Windows ships a broken python3 store stub.
PYBIN=""
for candidate in python3 python; do
	if $candidate -c "pass" >/dev/null 2>&1; then PYBIN=$candidate; break; fi
done
VERSION=$($PYBIN -c "import json;print(json.load(open('src-tauri/tauri.conf.json'))['version'])")
mkdir -p "$OUT"

CRATES=$($PYBIN - "$VERSION" <<'PY'
import re, sys
crates = []
lock = open('src-tauri/Cargo.lock', encoding='utf-8').read()
for name, ver in re.findall(r'\nname = "([^"]+)"\nversion = "([^"]+)"', lock):
    if name in ("vidaro",):
        continue
    crates.append(f"{name}-{ver}")
print(" ".join(dict.fromkeys(crates)))
PY
)

cat > "$OUT/vidaro-${VERSION}.ebuild" <<EOF
# Copyright 1999-2026 Gentoo Authors
# Distributed under the terms of the GNU General Public License v2

EAPI=8

CRATES="${CRATES}"

inherit cargo

DESCRIPTION="Desktop YouTube downloader built with Tauri 2 (Rust + React)"
HOMEPAGE="https://github.com/DashAkoul/vidaro"
SRC_URI="https://github.com/DashAkoul/vidaro/archive/refs/tags/v\${PV}.tar.gz -> \${P}.tar.gz
\${CARGO_CRATE_URIS}"

LICENSE="MIT"
SLOT="0"
KEYWORDS="~amd64"
IUSE=""

# The frontend is built with the bundled Node toolchain at compile time.
BDEPEND="
\tdev-util/pkgconf
\tnet-libs/nodejs[npm]
\tvirtual/rust
"
RDEPEND="
\tdev-libs/glib:2
\tnet-libs/webkit-gtk:4.1
\tdev-libs/libsoup:3.0
\tx11-libs/gtk+:3
"

S="\${WORKDIR}/\${P}"

src_configure() {
\t# The Rust crate lives in src-tauri/.
\texport CARGO_WORKDIR="\${S}/src-tauri"
\tnpm ci || die "npm ci failed"
\tcargo_src_configure
}

src_compile() {
\tnpm run build || die "frontend build failed"
\tcargo_src_compile
}

src_install() {
\tcargo_src_install
\tnewbin src-tauri/target/release/vidaro vidaro
\tdomenu packaging/vidaro.desktop || true
\tinsinto /usr/share/licenses/vidaro
\tdoins LICENSE
}
EOF

echo "wrote $OUT/vidaro-${VERSION}.ebuild ($(wc -l < "$OUT/vidaro-${VERSION}.ebuild") lines)"
