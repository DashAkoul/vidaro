#!/usr/bin/env bash
# Package the already-built Linux binary into an Arch Linux package
# (.pkg.tar.zst). Tauri's bundler does not produce Arch packages, and this
# script exists so the release ships one natively installable with
# `pacman -U`.
#
# Usage: scripts/package-arch.sh <version> [binary path]
set -euo pipefail

VERSION="${1:?usage: package-arch.sh <version> [binary]}"
BIN="${2:-src-tauri/target/release/vidaro}"

command -v zstd >/dev/null 2>&1 || (echo "zstd is required" && exit 1)
[ -x "$BIN" ] || (echo "binary not found: $BIN" && exit 1)

ROOT=".arch-pkg/vidaro"
rm -rf "$ROOT"
mkdir -p "$ROOT/usr/bin" \
         "$ROOT/usr/share/applications" \
         "$ROOT/usr/share/icons/hicolor/32x32/apps" \
         "$ROOT/usr/share/icons/hicolor/128x128/apps" \
         "$ROOT/usr/share/icons/hicolor/256x256/apps" \
         "$ROOT/usr/share/licenses/vidaro"

install -m755 "$BIN" "$ROOT/usr/bin/vidaro"
[ -f LICENSE ] && install -m644 LICENSE "$ROOT/usr/share/licenses/vidaro/LICENSE"

cat > "$ROOT/usr/share/applications/vidaro.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Vidaro
GenericName=YouTube Downloader
Comment=Download YouTube videos, playlists and channels
Exec=vidaro
Icon=vidaro
Terminal=false
Categories=Network;FileTransfer;
StartupWMClass=Vidaro
EOF

cp src-tauri/icons/32x32.png       "$ROOT/usr/share/icons/hicolor/32x32/apps/vidaro.png"
cp src-tauri/icons/128x128.png     "$ROOT/usr/share/icons/hicolor/128x128/apps/vidaro.png"
cp src-tauri/icons/256x256.png     "$ROOT/usr/share/icons/hicolor/256x256/apps/vidaro.png"

# .PKGINFO — the metadata pacman reads. Runtime libraries follow what Tauri
# links against on Linux; users install them through pacman automatically.
cat > "$ROOT/.PKGINFO" <<EOF
pkgname = vidaro
pkgver = ${VERSION}-1
pkgdesc = YouTube downloader for Windows, macOS and Linux
url = https://github.com/DashAkoul/vidaro
builddate = $(date +%s)
packager = Vidaro release workflow <https://github.com/DashAkoul/vidaro>
size = $(du -sb "$ROOT" | cut -f1)
arch = x86_64
license = MIT
depend = webkit2gtk-4.1
depend = gtk3
depend = glib2
depend = libsoup3
depend = openssl
depend = zstd

EOF

cd "$ROOT"
tar --use-compress-program="zstd -19" -cf "../../vidaro-${VERSION}-1-x86_64.pkg.tar.zst" .PKGINFO usr
cd - >/dev/null
rm -rf ".arch-pkg"
echo "wrote vidaro-${VERSION}-1-x86_64.pkg.tar.zst"
