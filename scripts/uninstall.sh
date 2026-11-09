#!/usr/bin/env bash
set -euo pipefail
if [[ $EUID == 0 ]]; then echo 'Run as your desktop user.' >&2; exit 1; fi
systemctl --user disable --now edgeslide.service || true
rm -f -- "$HOME/.local/bin/edgeslide-daemon" "$HOME/.local/bin/edgeslide-settings" "$HOME/.local/bin/edgeslide-indicator" \
 "$HOME/.config/systemd/user/edgeslide.service" \
 "$HOME/.local/share/applications/io.github.edgeslide.Settings.desktop" \
 "$HOME/.local/share/icons/hicolor/scalable/apps/io.github.edgeslide.Settings.svg"
rm -rf -- "$HOME/.local/share/doc/edgeslide"
systemctl --user daemon-reload
printf '%s\n' 'Uninstalled. User configuration and source checkout preserved.' 'If device permissions were installed, remove them with scripts/device-access.sh revoke USER TOUCHPAD_NAME as root.'
