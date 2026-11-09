#!/usr/bin/env bash
# Convenience installer for a local EdgeSlide checkout on Arch Linux.
set -euo pipefail

if [[ ${EUID} == 0 ]]; then
    echo "Run this installer as your desktop user, not root." >&2
    exit 1
fi

if [[ ! -f Cargo.toml || ! -x scripts/install.sh ]]; then
    echo "Run this from the root of an EdgeSlide checkout." >&2
    exit 1
fi

if ! command -v pacman >/dev/null; then
    echo "This convenience installer supports Arch Linux only." >&2
    echo "Install the dependencies listed in README.md, then run ./scripts/install.sh." >&2
    exit 1
fi

packages=(base-devel rust pkgconf gtk4 libadwaita libpulse pipewire wireplumber systemd python)
missing=()
for package in "${packages[@]}"; do
    pacman -Q "$package" >/dev/null 2>&1 || missing+=("$package")
done

if ((${#missing[@]})); then
    echo "Installing required packages: ${missing[*]}"
    sudo pacman -S --needed "${missing[@]}"
fi

./scripts/install.sh

cat <<'EOF'

EdgeSlide is installed for this user.

Start it now:       systemctl --user start edgeslide.service
Enable at login:    systemctl --user enable edgeslide.service
Open settings:      ~/.local/bin/edgeslide-settings

Touchpad read access is intentionally not granted by this script.
Review docs/PERMISSIONS.md before opting into that separate system change.
EOF
