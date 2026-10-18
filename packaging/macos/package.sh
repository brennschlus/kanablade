#!/usr/bin/env bash
# Builds dist/kanablade-<version>-macos.dmg: a universal (arm64 + x86_64) kanablade.app
# next to an Applications link, the usual drag-to-install window.
set -euo pipefail
cd "$(dirname "$0")/../.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
build=${GITHUB_RUN_NUMBER:-1}
name=kanablade-$version-macos
export MACOSX_DEPLOYMENT_TARGET=11.0

for target in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --locked --target $target
done

app=dist/dmg/kanablade.app
rm -rf dist/dmg "dist/$name.dmg"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/kanablade" \
    target/aarch64-apple-darwin/release/kanablade \
    target/x86_64-apple-darwin/release/kanablade
sed -e "s/@VERSION@/$version/" -e "s/@BUILD@/$build/" packaging/macos/Info.plist > "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist"
printf 'APPL????' > "$app/Contents/PkgInfo"
cp packaging/icons/kanablade.icns "$app/Contents/Resources/"
cp assets/fonts/OFL-ZenFonts.txt "$app/Contents/Resources/"

# Ad-hoc signature: required for arm64 and keeps the bundle sealed. Not notarized,
# so the first launch of a downloaded copy needs right-click → Open.
codesign --force --sign - "$app"
codesign --verify --strict "$app"

ln -s /Applications dist/dmg/Applications
hdiutil create -volname kanablade -srcfolder dist/dmg -fs HFS+ -format ULFO -ov "dist/$name.dmg"
rm -rf dist/dmg
echo "dist/$name.dmg"
