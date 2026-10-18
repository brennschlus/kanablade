#!/bin/sh
# Installs kanablade for the current user: binary, menu entry and icons.
#   ./install.sh               into ~/.local
#   PREFIX=/usr/local sudo -E ./install.sh
#   ./install.sh --uninstall
set -eu
cd "$(dirname "$0")"

id=io.github.brennschlus.kanablade
prefix=${PREFIX:-$HOME/.local}
files="bin/kanablade share/applications/$id.desktop share/metainfo/$id.metainfo.xml share/licenses/kanablade/OFL-ZenFonts.txt"
for png in share/icons/hicolor/*/apps/$id.png; do files="$files $png"; done

if [ "${1:-}" = "--uninstall" ]; then
    for f in $files; do rm -f "$prefix/$f"; done
    rmdir "$prefix/share/licenses/kanablade" 2>/dev/null || true
    echo "kanablade removed from $prefix"
else
    for f in $files; do
        mkdir -p "$prefix/$(dirname "$f")"
        cp "$f" "$prefix/$f"
    done
    # Absolute path, so the menu entry works even if $prefix/bin is not on PATH.
    sed -i "s|^Exec=kanablade|Exec=$prefix/bin/kanablade|" "$prefix/share/applications/$id.desktop"
    echo "kanablade installed into $prefix"
fi

# Let the menu and icon caches notice right away; harmless where the tools are missing.
update-desktop-database -q "$prefix/share/applications" 2>/dev/null || true
gtk-update-icon-cache -q "$prefix/share/icons/hicolor" 2>/dev/null || true
