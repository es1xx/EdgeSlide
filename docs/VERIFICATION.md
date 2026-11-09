# Verification — 2026-11-08

## Environment inspected

Arch Linux; Hyprland Wayland session; ELAN050A:00 04F3:3158 Touchpad
(101 × 73 mm), udev-classified touchpad at event16; AMD amdgpu_bl2 backlight,
65535 maximum; PipeWire 1.6.10 with pipewire-pulse and WirePlumber active;
GTK 4.24.1, libadwaita 1.10.0; Rust 1.99.1 installed into the user's home.

The touchpad initially had no user ACL. After explicit approval and local sudo
authentication, the reviewed helper installed an exact-device read-only ACL and
udev rule. No keyboard permissions or Hyprland configuration were changed.

## Checks executed

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --locked --workspace
for script in scripts/*.sh; do bash -n "$script"; done
systemd-analyze --user verify ~/.config/systemd/user/edgeslide.service
desktop-file-validate ~/.local/share/applications/io.github.edgeslide.Settings.desktop
~/.local/bin/edgeslide-daemon --check-config
~/.local/bin/edgeslide-daemon --list-devices
~/.local/bin/edgeslide-daemon --probe-backends
```

14 automated tests passed: configuration roundtrip/validation, edge activation,
center rejection, short-motion rejection, direction inversion, multitouch and
click/palm cancellation, leaving an edge, sensitivity scaling, evdev frame
boundaries/overflow suppression, and invalid indicator messages.

Native audio and logind brightness calls acknowledged same-value writes. The
hardware user then explicitly reported: **both edge gestures and normal input
work**, after being asked to check upward/downward slides on each edge, pointer
movement, tapping, two-finger scrolling, and the usual Hyprland gesture. This is
user-observed physical verification, not synthetic input injection.

Settings launched in the real Wayland session. Configuration changes and service
restarts were observed. The user selected left=brightness and right=volume and a
5% brightness floor; reinstall preserved those choices. Initial GUI close exited
with code 0. A screenshot was inspected. The current desktop GTK theme emitted
GtkImage baseline warnings; the settings window remained usable. No theme files
were changed.

The transient indicator mapped as a 300 × 86 layer-shell surface in all four
positions: top (810,32), bottom (810,962), left (32,497), right (1588,497) on the
1920 × 1080 display. Its screenshot was inspected. All four runs exited with
code 0 without stderr, about 0.90 seconds after the last update. A streamed
value update extended the same process lifetime to 1.41 seconds, as expected.
Preview does not change volume/brightness. Normal application Quit also exited
with code 0. A duplicate daemon was safely refused; stop/start returned the
service to active/running. Autostart was enabled through the user's settings.

## Actual idle measurement

A 30.0001-second interval was measured after the daemon opened the real touchpad,
with the touchpad left untouched. Reads were taken only at interval endpoints.

| Metric | Observed |
|---|---:|
| CPU ticks added (100 ticks/second clock) | 0 |
| CPU utilization at tick resolution | 0.0% of one CPU |
| Summed thread scheduler runtime added | 0 ns |
| Voluntary/involuntary context switches added | 0 / 0 |
| RSS | 8396 KiB (8.20 MiB) |
| Proportional memory (PSS) | 2444 KiB (2.39 MiB) |
| Threads | 2 |

The input thread slept in `poll_schedule_timeout.constprop.0`; the action worker
slept in `__futex_wait`. Neither changed scheduler counters during this interval.
This is evidence of no observed daemon wakeups in that sample, **not** a whole-
system power measurement or a guarantee across workloads. The measurement was
made before the optional overlay was added; with feedback disabled, the overlay
adds no running process. Final-build measurement follows below.
Raw local evidence and screenshots are in ignored `.agents/reports/`.

## Final installed build measurement

`./scripts/measure-idle.py --seconds 30 --output .agents/reports/idle-final.json`
measured the final installed daemon with the touchpad open and feedback disabled:

| Metric | Observed |
|---|---:|
| Interval | 30.0001 seconds |
| CPU ticks added | 0 (0.0% at 10 ms tick resolution) |
| Thread scheduler runtime added | 24,676 ns (about 0.000082% of one CPU) |
| Voluntary/involuntary context switches added | 2 / 0 |
| RSS | 8468 KiB (8.27 MiB) |
| PSS | 2503 KiB (2.44 MiB) |
| Threads | 2 |

Two scheduling wakeups were observed during this sample; their cause was not
traced. No periodic application timer exists on this idle path. This sample was
taken after service restart, before new actions established native backend
connections; memory and thread counts can increase after backend initialization.
The measurement script is included for repeating a warm-idle measurement after
normal use. It takes endpoint samples and sleeps between them.

## Remaining manual checks

- Suspend and resume: leave no finger down, suspend normally, resume/unlock,
  test both edges and normal input. No forced suspend was performed.
- Real hardware disconnect/reconnect and a kernel-generated SYN_DROPPED stream
  were not forced. Software recovery paths and overflow suppression are present.
- Long-term palm rejection, disable-while-typing interactions, and other touchpad
  models require further use. EdgeSlide does not read the keyboard, so it does
  not mirror libinput's typing suppression.
- Import/export file chooser interaction needs a final hands-on UI check. TOML
  parsing, validation, and serialization are tested. All appearance placements
  and streamed indicator updates were checked in the real Wayland session.
- No battery wattage, energy-per-gesture, or broad benchmark claims are made.

## Short regression procedure

1. Check the journal and service state. Lift all fingers and slide each edge up
   and down. Confirm the configured action and direction.
2. Start in the center and move into an edge: no adjustment should start. Make
   short edge motions below the threshold, then try two fingers and a physical
   click: no adjustment should continue until all fingers lift.
3. Verify pointer motion, taps, clicks, scrolling, and existing Hyprland gestures.
4. Save changed edge actions, restart, and confirm they persist. Export, import,
   then restore a saved configuration. Preview the indicator in each position.
5. Close settings; gestures should keep working and no settings process should
   remain. The indicator should disappear within about one second of inactivity.
6. Perform the suspend/resume check above when convenient.
