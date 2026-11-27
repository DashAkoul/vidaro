#!/bin/ash
# Build Vidaro for Alpine Linux and package it as an .apk.
#
# Runs INSIDE an alpine:3.21 container (see build.yml — build-alpine).
# The full Tauri GUI is compiled against musl + the same webkit2gtk-4.1
# the app uses everywhere else, then abuild wraps the finished binary into
# an installable package. Output lands in /work/dist-alpine/.
set -e

echo "== installing the build toolchain"
apk add --no-cache   build-base pkgconf linux-headers openssl-dev   gtk+3.0-dev webkit2gtk-4.1-dev   glib-dev gdk-pixbuf-dev cairo-dev pango-dev gettext-dev harfbuzz-dev   nodejs npm zstd curl   abuild sudo git

# The distro's Rust (1.83 in Alpine 3.21) cannot parse crates that use
# edition 2024, so install the current stable through rustup instead.
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
export PATH="$HOME/.cargo/bin:$PATH"
rustc --version

# The musl target defaults to a static PIE, which cannot link the shared
# GTK/WebKit libraries this app needs — force dynamic linking instead.
export RUSTFLAGS="-C target-feature=-crt-static"

echo "== building the frontend"
npm ci
npm run build

echo "== compiling the Rust binary against musl"
cd src-tauri
cargo build --release
cd ..

VERSION=$(sed -n 's/.*"version": "\([^"]*\)".*/\1/p' src-tauri/tauri.conf.json | head -1)
echo "== packaging vidaro-${VERSION}"

# abuild insists on a non-root builder with sudo rights.
adduser -D builder
addgroup builder abuild
printf 'builder ALL=(ALL) NOPASSWD: ALL\n' > /etc/sudoers.d/builder

HOME_B=/home/builder
mkdir -p "$HOME_B/apk/vidaro" "$HOME_B/packages"
cp src-tauri/target/release/vidaro "$HOME_B/apk/vidaro/"
cp packaging/vidaro.desktop "$HOME_B/apk/vidaro/"
cp src-tauri/icons/32x32.png "$HOME_B/apk/vidaro/vidaro-32.png"
cp src-tauri/icons/128x128.png "$HOME_B/apk/vidaro/vidaro-128.png"
cp LICENSE "$HOME_B/apk/vidaro/LICENSE" 2>/dev/null || true

cat > "$HOME_B/apk/vidaro/APKBUILD" <<APKEOF
# Maintainer: DashAkoul <dashakoul@users.noreply.github.com>
pkgname=vidaro
pkgver=${VERSION}
pkgrel=0
pkgdesc="Desktop YouTube downloader built with Tauri 2"
url="https://github.com/DashAkoul/vidaro"
arch="x86_64"
license="MIT"
depends="gtk+3.0 webkit2gtk-4.1"
makedepends=""
source=""
options="!check"

prepare() {
	default_prepare
	cp "\$startdir"/vidaro "\$srcdir"/vidaro
	cp "\$startdir"/vidaro.desktop "\$srcdir"/vidaro.desktop
	cp "\$startdir"/vidaro-32.png "\$srcdir"/vidaro-32.png
	cp "\$startdir"/vidaro-128.png "\$srcdir"/vidaro-128.png
}

build() {
	return 0
}

package() {
	install -Dm755 "\$srcdir"/vidaro "\$pkgdir"/usr/bin/vidaro
	install -Dm644 "\$srcdir"/vidaro.desktop \
		"\$pkgdir"/usr/share/applications/vidaro.desktop
	install -Dm644 "\$srcdir"/vidaro-32.png \
		"\$pkgdir"/usr/share/icons/hicolor/32x32/apps/vidaro.png
	install -Dm644 "\$srcdir"/vidaro-128.png \
		"\$pkgdir"/usr/share/icons/hicolor/128x128/apps/vidaro.png
}
APKEOF

chown -R builder:builder "$HOME_B"

su builder -c "cd ~/apk/vidaro && abuild-keygen -a -n -i && abuild -r"

APK=$(find "$HOME_B/packages" -name "vidaro-${VERSION}*.apk" | head -1)
[ -n "$APK" ] || { echo "no apk produced"; exit 1; }
mkdir -p dist-alpine
cp "$APK" "dist-alpine/vidaro-${VERSION}-r0.apk"
echo "wrote dist-alpine/vidaro-${VERSION}-r0.apk"
