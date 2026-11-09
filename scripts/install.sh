#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
if [[ $EUID == 0 ]]; then echo 'Run as your desktop user, not root.' >&2; exit 1; fi
cargo_bin="${CARGO:-$HOME/.cargo/bin/cargo}"
if [[ ! -x "$cargo_bin" ]]; then cargo_bin=cargo; fi
"$cargo_bin" build --release --locked --workspace
install -Dm755 target/release/edgeslide-daemon "$HOME/.local/bin/edgeslide-daemon"
install -Dm755 target/release/edgeslide-indicator "$HOME/.local/bin/edgeslide-indicator"
install -Dm755 target/release/edgeslide-settings "$HOME/.local/bin/edgeslide-settings"
install -Dm644 data/edgeslide.service "$HOME/.config/systemd/user/edgeslide.service"
install -Dm644 data/io.github.edgeslide.Settings.svg "$HOME/.local/share/icons/hicolor/scalable/apps/io.github.edgeslide.Settings.svg"
install -Dm644 data/io.github.edgeslide.Settings.desktop "$HOME/.local/share/applications/io.github.edgeslide.Settings.desktop"
# An absolute executable path works even when ~/.local/bin is not on the GUI session PATH.
python3 - "$HOME/.local/share/applications/io.github.edgeslide.Settings.desktop" "$HOME/.local/bin/edgeslide-settings" <<'PYCODE'
import pathlib,sys
p=pathlib.Path(sys.argv[1]); executable=sys.argv[2].replace('\\','\\\\').replace('"','\\"').replace('`','\\`').replace('$','\\$').replace('%','%%')
p.write_text(p.read_text().replace('Exec=edgeslide-settings','Exec="'+executable+'"'))
PYCODE
install -Dm644 README.md "$HOME/.local/share/doc/edgeslide/README.md"
install -Dm644 LICENSE "$HOME/.local/share/doc/edgeslide/LICENSE"
for doc in docs/*.md; do install -Dm644 "$doc" "$HOME/.local/share/doc/edgeslide/$doc"; done
config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/edgeslide"
mkdir -p "$config_dir"
if [[ ! -e "$config_dir/config.toml" ]]; then install -m600 data/config.toml "$config_dir/config.toml"; fi
systemctl --user daemon-reload
printf '%s\n' 'Installed. Existing configuration preserved. Autostart is unchanged.' 'Launch: ~/.local/bin/edgeslide-settings' 'Start: systemctl --user start edgeslide.service' 'Device permissions: see docs/PERMISSIONS.md'
