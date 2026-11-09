<div align="center">
  <img src="data/io.github.edgeslide.Settings.svg" width="128" alt="EdgeSlide icon">
  <h1>EdgeSlide</h1>
  <p><strong>Touchpad edge controls and native gestures for Hyprland.</strong></p>
  <p>Rust · GTK4/libadwaita · PipeWire · systemd user service</p>
</div>

<p align="center">
  <a href="LICENSE">MIT License</a> ·
  <a href="#install-on-arch-linux">Install</a> ·
  <a href="#gestures">Gestures</a> ·
  <a href="docs/PERMISSIONS.md">Device access</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust 2021">
  <img src="https://img.shields.io/badge/platform-Linux-FCC624?style=flat-square&logo=linux&logoColor=111" alt="Linux">
  <img src="https://img.shields.io/badge/compositor-Hyprland-58E1FF?style=flat-square" alt="Hyprland">
  <img src="https://img.shields.io/badge/interface-GTK4%20%2B%20libadwaita-4A86CF?style=flat-square" alt="GTK4 and libadwaita">
  <img src="https://img.shields.io/badge/license-MIT-86E1A7?style=flat-square" alt="MIT License">
</p>

EdgeSlide makes the edges of a laptop touchpad useful without taking control of
the touchpad away from Hyprland. It runs as a small user service; the GTK
settings window and visual indicator are short-lived processes.

| Gesture | Default action |
| --- | --- |
| One finger, left edge, up/down | Volume up/down |
| One finger, right edge, up/down | Brightness up/down |
| Four fingers, left/right | Previous/next workspace |
| Three fingers, down | Area screenshot |
| Three fingers, up | Optional overview command |

Every gesture is configurable or can be disabled from the settings app.

## See it in action

<p align="center">
  <img src="assets/settings-preview.png" alt="EdgeSlide settings window on Hyprland" width="920">
</p>

<p align="center"><em>One compact settings window for edge controls, native gestures, feedback and diagnostics.</em></p>

<p align="center">
  <img src="assets/indicator-preview.png" alt="EdgeSlide brightness indicator" width="300">
</p>

<p align="center"><em>Optional click-through feedback disappears automatically when the gesture ends.</em></p>

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

## Install on Arch Linux

```sh
git clone https://github.com/es1xx/EdgeSlide.git
cd EdgeSlide
./scripts/install-arch.sh
```

The convenience installer checks for the required packages, installs only missing
ones with `pacman`, then builds the locked Rust workspace and installs EdgeSlide
under `~/.local`. It preserves existing configuration and does not enable
autostart on its own. It must be run as your desktop user, not root.

If the build dependencies are already present, use `./scripts/install.sh`.

Touchpad read access is intentionally separate. Read [device access](docs/PERMISSIONS.md)
before explicitly granting it; EdgeSlide never silently broadens input permissions.

```sh
systemctl --user start edgeslide.service
systemctl --user enable edgeslide.service       # start on future logins
systemctl --user disable --now edgeslide.service
~/.local/bin/edgeslide-settings
```

## Gestures

Open **EdgeSlide → Gestures** to configure the one-finger edge controls and the
separate native multi-finger section. Multi-finger actions use Hyprland's Lua
gesture API, so they add no second daemon, input reader or idle timer.

Three-finger up is disabled until you enter a command for an overview tool you
have already installed. EdgeSlide does not assume or install an overview provider.
See [multi-finger setup](docs/MULTIFINGER.md) for Hyprland integration, conflict
handling and custom actions.

## Safety and behavior

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
