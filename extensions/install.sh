#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
target="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/cosnip@cosnip"

mkdir -p "$(dirname "$target")"
rm -rf "$target"
cp -r "$here/cosnip@cosnip" "$target"
glib-compile-schemas "$target/schemas"

if command -v gnome-extensions >/dev/null 2>&1; then
    gnome-extensions disable cosnip-pins@cosnip >/dev/null 2>&1 || true
    gnome-extensions enable cosnip@cosnip >/dev/null 2>&1 || true
fi

# New extensions are only scanned by the Shell at login. Record the new UUID
# and drop the legacy pin extension from the enabled list.
if command -v gsettings >/dev/null 2>&1; then
    enabled="$(gsettings get org.gnome.shell enabled-extensions 2>/dev/null || echo '[]')"
    enabled="$(printf '%s' "$enabled" | sed "s/, 'cosnip-pins@cosnip'//g; s/'cosnip-pins@cosnip', //g")"
    if ! printf '%s' "$enabled" | grep -q 'cosnip@cosnip'; then
        case "$enabled" in
            "[]"|"@as []"|"")
                enabled="['cosnip@cosnip']"
                ;;
            *)
                enabled="$(printf '%s' "$enabled" | sed "s/]$/, 'cosnip@cosnip']/")"
                ;;
        esac
        gsettings set org.gnome.shell enabled-extensions "$enabled" || true
    fi
fi

cat <<EOF
Installed the native Cosnip extension to:
  $target

Log out and back in so GNOME Shell scans the new extension, then check:
  gnome-extensions info cosnip@cosnip
  gsettings get org.gnome.shell.extensions.cosnip snip-shortcut

Defaults: F1 snip, F3 paste-as-pin.
Preferences: gnome-extensions prefs cosnip@cosnip
EOF
