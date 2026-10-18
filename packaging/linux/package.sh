#!/usr/bin/env bash
# Builds dist/kanablade-<version>-linux-<arch>.tar.gz: a ~/.local-shaped tree
# (bin/, share/applications, share/icons/hicolor, share/metainfo) plus install.sh.
set -euo pipefail
cd "$(dirname "$0")/../.."

id=io.github.brennschlus.kanablade
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
arch=$(uname -m)
name=kanablade-$version-linux-$arch
root=dist/$name

cargo build --release --locked

rm -rf "$root"
install -Dm755 target/release/kanablade "$root/bin/kanablade"
install -Dm755 packaging/linux/install.sh "$root/install.sh"
install -Dm644 packaging/linux/$id.desktop "$root/share/applications/$id.desktop"
install -Dm644 packaging/linux/$id.metainfo.xml "$root/share/metainfo/$id.metainfo.xml"
install -Dm644 assets/fonts/OFL-ZenFonts.txt "$root/share/licenses/kanablade/OFL-ZenFonts.txt"
for png in packaging/icons/hicolor/*/apps/$id.png; do
    install -Dm644 "$png" "$root/share/icons/${png#packaging/icons/}"
done

tar -C dist -czf "dist/$name.tar.gz" --owner=0 --group=0 "$name"
echo "dist/$name.tar.gz"
