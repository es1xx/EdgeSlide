# EdgeSlide

A small Rust touchpad edge gesture controller for Linux, with GTK4/libadwaita
settings. Slide upward along the left edge for volume and the right edge for
brightness. Actions, edge widths, thresholds, speeds, limits, and directions are
configurable independently. MIT licensed; the root license covers project code
and artwork, while dependencies retain their own licenses.

## Architecture

- `edgeslide-core`: validated TOML configuration, evdev frame decoding, gesture state machine.
- `edgeslide-daemon`: nonexclusive read-only evdev observer, udev hotplug monitor,
  blocking poll, native libpulse control through PipeWire-Pulse, logind brightness.
- `edgeslide-settings`: GTK4/libadwaita settings, service controls, diagnostics,
  import/export, defaults. Closing the window releases the entire GUI process.
  Its `edgeslide-indicator` helper draws an optional click-through Wayland overlay
  and exits after 900 ms without updates; no persistent GTK process is used.

There is no input grab, injected input, compositor reconfiguration, root daemon,
idle polling timer, or per-adjustment shell command. A sleeping action worker
coalesces changes while gestures are active. Native audio requests have bounded
timeouts. Device ranges and the backlight maximum are cached. Default audio sink
and its current level are re-read for each adjustment to respect external changes.
Configuration is loaded at startup; Save and apply restarts an already-running
service. Disabled gestures do not open input devices.

## Build and install (Arch Linux)

Dependencies: Rust with cargo/rustfmt/clippy, `base-devel`, `pkgconf`, `gtk4`,
`libadwaita`, `libpulse`, `systemd`, `pipewire-pulse`, `wireplumber`, `python`,
and `acl` if installing the optional permission rule. Existing desktop packages
must be inspected before installing anything. GTK >= 4.12 and libadwaita >= 1.4.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/install.sh
~/.local/bin/edgeslide-settings
```

The installer builds with `Cargo.lock`, installs under `~/.local`, preserves
existing `~/.config/edgeslide/config.toml`, and does not change autostart.
No privileged operation is performed by it. With a custom XDG_CONFIG_HOME,
set the same environment in the user service; otherwise use the default path.

```sh
systemctl --user start edgeslide.service
systemctl --user enable edgeslide.service       # start on future logins
systemctl --user disable --now edgeslide.service
journalctl --user -u edgeslide.service -n 30
~/.local/bin/edgeslide-daemon --list-devices
~/.local/bin/edgeslide-daemon --check-config
~/.local/bin/edgeslide-daemon --probe-backends    # explicit same-value write test
```

Read [device access](docs/PERMISSIONS.md) before granting permissions. A service
can be active while waiting for access; the journal explains that state. No
physical gesture will work until the selected touchpad is readable.

## Gesture behavior and limitations

A finger must start inside an edge. Movement beyond the activation threshold is
consumed to avoid a sudden jump. Subsequent vertical movement is proportional
to touchpad height. Multiple fingers, physical clicks, reported palms, sideways
exit, stale events, and interrupted contacts cancel until release. A held contact
with no reports for 750 ms is cancelled. After an input overflow or startup with a finger already down, lift
all fingers once before retrying. Type-B hardware must expose ABS_X/Y and
BTN_TOUCH/BTN_TOOL_FINGER compatibility state; unsupported devices are rejected.
Palm recognition is conservative and depends on hardware tool reporting; this
observer cannot reproduce all libinput palm/typing heuristics.

Normal pointer movement, taps, clicks, scrolling and Hyprland gestures remain
under compositor control. An edge gesture can still move the pointer or produce
a tap. Keyboard-driven disable-while-typing is not mirrored because EdgeSlide
does not read keyboard devices. Tune narrow edges and sufficient thresholds.

Brightness defaults to a 10% minimum, with a hard configurable lower bound of 5%.
Validate readability on your panel before reducing it. Audio preserves channel
balance and mute state; it does not unmute automatically. The first alphabetic
backlight device is selected if multiple backlights exist.

Optional feedback uses the Wayland layer-shell protocol supported by Hyprland.
Choose top, bottom, left, or right placement and blue, amber, or green accents.
The overlay never accepts keyboard/pointer input or reserves screen space. It
uses double-buffered shared memory and redraws only on value updates. Its one-shot
inactivity timeout closes it after 900 ms. Unsupported compositors cannot display
this optional overlay. The user systemd environment must contain WAYLAND_DISPLAY
and XDG_RUNTIME_DIR for it to launch; these were already present on the inspected
Hyprland session. Preview saved appearance in General settings before enabling it.

## Verification and removal

Repeat the endpoint-only idle measurement with `./scripts/measure-idle.py --seconds 30`.
Keep the touchpad untouched during the interval; run it after normal gestures to
include initialized backends.

See [verification](docs/VERIFICATION.md) for actual results and the manual
hardware checklist. Do not treat unit tests as proof of physical gesture behavior.

```sh
./scripts/uninstall.sh
```

This preserves user configuration and the source checkout. Revoke the optional
root-installed touchpad rule separately as described in PERMISSIONS.md.
The project can be published from its local Git repository when the maintainer is ready.

## Protocol references

- [Linux input event and SYN_DROPPED semantics](https://www.kernel.org/doc/html/latest/input/event-codes.html)
- [Linux multitouch protocol](https://www.kernel.org/doc/html/latest/input/multi-touch-protocol.html)
- [logind session and brightness APIs](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html)
- [Wayland layer-shell protocol](https://github.com/swaywm/wlr-protocols/blob/master/unstable/wlr-layer-shell-unstable-v1.xml)

## Optional multi-finger gestures

The existing Gestures tab also offers an independent native Hyprland section for
four-finger workspace navigation, three-finger area screenshots, and configurable
custom/overview commands. It uses a separate configuration file and leaves the
original edge daemon unchanged. See [native multi-finger integration](docs/MULTIFINGER.md)
for setup, conflict handling, independent enable/disable controls, and tests.
