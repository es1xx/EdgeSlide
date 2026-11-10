# EdgeSlide guide

## Installation and lifecycle

The Arch convenience installer checks for missing packages before requesting
pacman. The generic source installer expects the dependencies to be present.
Both build with the Rust lockfile, install into ~/.local and preserve existing
configuration. Run as the desktop user, not root.

Read [touchpad access](PERMISSIONS.md) before granting access to the device.
Native multi-finger gestures require the separate opt-in setup described in
[MULTIFINGER.md](MULTIFINGER.md), including a compatible Hyprland Lua configuration.

The edge daemon and native compositor gestures are independently enabled.
Stopping the daemon does not disable already-installed native mappings.

```bash
systemctl --user start edgeslide.service
systemctl --user enable edgeslide.service
systemctl --user disable --now edgeslide.service
~/.local/bin/edgeslide-settings
```

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

## Edge behavior


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
and XDG_RUNTIME_DIR for it to launch. Preview saved appearance in General settings before enabling it.

## Verification and removal

The [edge report](VERIFICATION.md) and
[multi-finger report](MULTIFINGER-VERIFICATION.md) distinguish automated tests,
physical checks and remaining limits. Reproduce warm-idle observations after
normal gestures using:

```bash
./scripts/measure-idle.py --seconds 30
```

Keep the touchpad untouched during the measurement. This is a process activity
measurement, not a battery benchmark.

```bash
./scripts/uninstall.sh
```

Removal preserves user configuration and the source checkout. Revoke the optional
root-installed device rule separately, following [PERMISSIONS.md](PERMISSIONS.md).

## README capture

The settings screenshot is a real 760 x 900 application window, captured with
its standard libadwaita styling in a temporary XDG_CONFIG_HOME.
A temporary compositor rule made that window floating and opaque.
No user settings were saved, no service buttons were pressed, and the temporary
window and compositor rule were removed afterward. The indicator image is the
existing real overlay preview.

## Protocol references


- [Linux input event and SYN_DROPPED semantics](https://www.kernel.org/doc/html/latest/input/event-codes.html)
- [Linux multitouch protocol](https://www.kernel.org/doc/html/latest/input/multi-touch-protocol.html)
- [logind session and brightness APIs](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html)
- [Wayland layer-shell protocol](https://github.com/swaywm/wlr-protocols/blob/master/unstable/wlr-layer-shell-unstable-v1.xml)
