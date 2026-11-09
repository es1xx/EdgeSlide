# Native multi-finger gestures

This additive extension uses Hyprland's native Lua gesture callbacks (tested with
0.57.0). It adds no daemon, input reader, timer, or polling loop. The original
EdgeSlide daemon, one-finger decoder, recognizer, and config.toml are unchanged.

Default mappings:

| Gesture | Action |
|---|---|
| Four fingers left | Previous workspace (`-1`) |
| Four fingers right | Next workspace (`+1`) |
| Three fingers down | Area screenshot |
| Three fingers up | Disabled until an overview provider is configured |

Workspace boundaries follow Hyprland's relative workspace selection. A completed
swipe fires once after crossing a distance threshold. Cancelled, reversed,
diagonal, excessively short/long and changed-finger gestures are rejected.
Sensitivity adjusts the distance threshold only; no global Hyprland sensitivity
or animation option is modified. Modifiers must be released.

## Settings

Open EdgeSlide and select the existing **Gestures** tab. The appended
**Multi-finger gestures** section has an independent master switch, per-gesture
switches, action selectors, sensitivity, and command fields. Its own **Save
multi-finger gestures** button saves and applies these mappings without restarting
the daemon. **Restore gesture mappings** resets only these new controls; save to
apply. The original save/defaults controls keep their previous scope.

New settings live in `~/.config/edgeslide/gestures.toml` (or XDG_CONFIG_HOME).
Existing `config.toml` is neither migrated nor rewritten by this extension.
The original edge-control enable switch and user service do not control these
native compositor gestures; use the separate multi-finger master switch.

Commands execute as your desktop user after a completed swipe. Custom actions
require a command. Overview requires an explicit command for a provider that you
have installed; no plugin is installed automatically. The inspected system has
no overview provider, so three-finger up is intentionally inactive. Screenshot
commands can be overridden. This installation uses the existing MacShell
`screenshot region` IPC command. Generic defaults use slurp and grim, save to
Pictures/Screenshots, and do not capture anything when selection is cancelled.

## Installation and lifecycle

The normal project installer includes the one-shot `edgeslide-gestures` helper.
To opt into native integration on a compatible Hyprland Lua setup:

```sh
~/.local/bin/edgeslide-gestures generate
~/.local/bin/edgeslide-gestures source-line
# Review the printed line, then append it once, or use the backup-making helper:
~/.local/bin/edgeslide-gestures install-source ~/.config/hypr/hyprland.lua
~/.local/bin/edgeslide-gestures apply
```

`install-source` appends one guarded source line and saves the original file as
`hyprland.lua.edgeslide-backup-PID`. It never replaces existing lines. Hyprland may
automatically reload when its configuration changes. The generated loader reads
`native-mappings.lua` and `native-engine.lua` in EdgeSlide's config directory.
Repeated apply replaces only EdgeSlide's exact registered tuples, including its
reserved scale marker `1.000739`; it does not clear native gesture tables or
rewrite bindings. Lua state and native registrations reset together on compositor
reload, so the source line restores the mappings without accumulating callbacks.
Saving the new settings reloads Hyprland to apply the generated mappings; it does
not restart EdgeSlide's daemon.

Hyprland rejects a new mapping shadowed by an existing matching direction, axis,
or all-swipe gesture. The apply command reports that error; it never forcibly
unsets the existing gesture. An error does not imply all mappings were applied.
Resolve conflicts explicitly in your own configuration or disable the conflicting
EdgeSlide mapping. Do not use EdgeSlide's reserved scale marker in other bindings.
A failed registration can also produce a harmless "non-existent gesture" error
when later clearing its reserved tuple; no foreign tuple is removed.

To disable all native mappings immediately without touching saved choices:

```sh
~/.local/bin/edgeslide-gestures clear
```

This also writes an empty generated mapping list for subsequent reloads. Saving
in settings or running `apply` restores saved choices. Uninstall calls `clear`
and removes the helper. The guarded source line may be removed manually; if its
loader is absent it does nothing. If uninstall is run outside a Hyprland session,
reload the compositor configuration afterward to discard live registrations.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
lua tests/multifinger.lua
```

The Rust suite includes the unchanged original tests plus independent settings
validation/serialization/escaping tests. The Lua harness models the dispatcher
as a non-callable descriptor, native conflicts, exact tuple removal, cancellation,
short/reversed/diagonal swipes, finger-count changes, sensitivity, duplicate end
events, repeated application, in-flight replacement and custom/screenshot actions.
It runs without changing the desktop.

The initial physical test exposed incorrect direct invocation of a dispatcher
object; workspace actions were corrected to `hl.dispatch(hl.dsp.focus(...))` and
the harness was updated to catch that failure. See the local verification report
for the final runtime and physical-test results.

Manual regression: swipe right then left with four fingers; confirm one workspace
change per swipe. Swipe down with three fingers and cancel/select an area. Verify
normal scrolling and the existing one-finger volume/brightness edges. Open settings,
change only a new mapping, save twice, and verify one activation per swipe.

References: [Hyprland gestures](https://wiki.hypr.land/configuring/core/binds/gestures/)
and the [0.57.0 native recognizer](https://github.com/hyprwm/Hyprland/blob/v0.57.0/src/managers/input/trackpad/TrackpadGestures.cpp).
