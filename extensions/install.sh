#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
target="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/cosnip-pins@cosnip"

mkdir -p "$(dirname "$target")"
rm -rf "$target"
cp -r "$here/cosnip-pins@cosnip" "$target"

if command -v gnome-extensions >/dev/null 2>&1; then
    gnome-extensions enable cosnip-pins@cosnip || true
fi

# gnome-extensions cannot enable a newly copied extension until the Shell has
# scanned it (normally at login), so also record it in the enabled list.
if command -v gsettings >/dev/null 2>&1; then
    enabled="$(gsettings get org.gnome.shell enabled-extensions 2>/dev/null || echo '[]')"
    if ! printf '%s' "$enabled" | grep -q 'cosnip-pins@cosnip'; then
        case "$enabled" in
            "[]"|"@as []"|"")
                enabled="['cosnip-pins@cosnip']"
                ;;
            *)
                enabled="$(printf '%s' "$enabled" | sed "s/]$/, 'cosnip-pins@cosnip']/")"
                ;;
        esac
        gsettings set org.gnome.shell enabled-extensions "$enabled" || true
    fi
fi

cat <<EOF
Installed Cosnip Pins to:
  $target

If the extension does not appear immediately, log out and back in
(GNOME on Wayland cannot reload the Shell in place), then run:
  gnome-extensions enable cosnip-pins@cosnip

Cosnip uses it automatically when it is running:
  [pin]
  backend = "auto"    # auto | shell | window
EOF
