#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
target="$data_home/gnome-shell/extensions/cosnip@choco2233"
old_native="$data_home/gnome-shell/extensions/cosnip@cosnip"
legacy="$data_home/gnome-shell/extensions/cosnip-pins@cosnip"

mkdir -p "$(dirname "$target")"
rm -rf "$old_native" "$legacy" "$target"
cp -r "$here/cosnip@choco2233" "$target"
glib-compile-schemas "$target/schemas"

if command -v gnome-extensions >/dev/null 2>&1; then
    gnome-extensions disable cosnip-pins@cosnip >/dev/null 2>&1 || true
    gnome-extensions disable cosnip@cosnip >/dev/null 2>&1 || true
    gnome-extensions enable cosnip@choco2233 >/dev/null 2>&1 || true
fi

# GNOME Shell only rescans extensions at login. Migrate the enabled list from
# the previous UUID and drop the legacy pin extension.
if command -v gsettings >/dev/null 2>&1; then
    enabled="$(gsettings get org.gnome.shell enabled-extensions 2>/dev/null || echo '[]')"
    enabled="$(printf '%s' "$enabled" | sed "s/'cosnip-pins@cosnip'//g; s/'cosnip@cosnip'/'cosnip@choco2233'/g")"
    enabled="$(printf '%s' "$enabled" | sed 's/\[ *,/[/g; s/, *,/,/g; s/, *\]/]/g')"
    if ! printf '%s' "$enabled" | grep -q 'cosnip@choco2233'; then
        case "$enabled" in
            "[]"|"@as []"|"")
                enabled="['cosnip@choco2233']"
                ;;
            *)
                enabled="$(printf '%s' "$enabled" | sed "s/]$/, 'cosnip@choco2233']/")"
                ;;
        esac
    fi
    gsettings set org.gnome.shell enabled-extensions "$enabled" || true

    disabled="$(gsettings get org.gnome.shell disabled-extensions 2>/dev/null || echo '[]')"
    disabled="$(printf '%s' "$disabled" | sed "s/'cosnip-pins@cosnip'//g; s/'cosnip@cosnip'//g")"
    disabled="$(printf '%s' "$disabled" | sed 's/\[ *,/[/g; s/, *,/,/g; s/, *\]/]/g')"
    gsettings set org.gnome.shell disabled-extensions "$disabled" || true
fi

cat <<EOF
Installed the native Cosnip extension to:
  $target

Log out and back in so GNOME Shell scans the new extension, then check:
  gnome-extensions info cosnip@choco2233
  gsettings get org.gnome.shell.extensions.cosnip snip-shortcut

Defaults: F1 snip, F3 paste-as-pin.
Preferences: gnome-extensions prefs cosnip@choco2233
EOF
