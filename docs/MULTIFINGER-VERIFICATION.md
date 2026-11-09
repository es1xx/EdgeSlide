# Multi-finger extension verification — 2026-11-09

Implemented as separate Rust configuration/UI modules, a one-shot integration
helper, and native Hyprland Lua callbacks. No evdev extension or second recognizer
was added to the daemon.

## Preservation evidence

SHA-256 comparisons before and after implementation matched for every original
daemon source file, the one-finger recognizer and decoder, the original configuration
model, the installed daemon executable, and the user's existing config.toml.
The daemon remained active with PID 784267 throughout deployment. It was neither
restarted nor replaced. All 37 Hyprland key bindings compared equal before/after.
The original Hyprland Lua file remains a verbatim prefix of the current file;
only one guarded source line was appended, with an original-file backup.

Existing-source changes are limited to exporting the new core module, registering
the new settings module, one call appending its group to the existing Gestures
page, and installation/removal of the new helper. Existing functionality was not
refactored. Original default values and configuration files are unchanged.

## Tests and live checks

- All 19 Rust tests pass, including all 14 pre-existing tests.
- `cargo fmt --all -- --check` and warnings-as-errors Clippy pass.
- `lua tests/multifinger.lua` passes recognition and lifecycle regression tests.
- Build succeeds with the existing dependency lockfile; no dependency was added.
- Native application succeeds and reports no Hyprland configuration errors.
- Reapplying retains exactly the three enabled mappings without accumulating
  registrations; overview remains disabled.
- Live callback exercise switched workspace 2 → 3 → 2. Duplicate finish events
  produced no extra switch. The original workspace was restored.
- The user physically retested after the dispatcher fix and confirmed:
  **all new gestures and original edges work**. The prompt explicitly covered
  four-finger navigation, three-finger area selection, and original edge controls.
- Updated settings window mapped successfully and exited normally via Quit.
- Script syntax and Git whitespace checks pass. Nothing was published or pushed.

An initial physical test found incorrect direct invocation of a dispatcher
object. It was fixed using `hl.dispatch`, and both the mock representation and
live checks were strengthened to cover the error. The final physical test passed.

## Limits

No overview provider is installed, so three-finger up is intentionally inactive.
An explicit installed-provider command is required to enable that mapping.
Foreign gesture conflict preservation is covered by matching upstream semantics
and the Lua harness; no existing live user gesture was overwritten to test it.
No forced suspend/resume or long-term battery benchmark was performed.

No new background service or process remains after applying settings. Native
callbacks run only during matching gestures; there are no added idle timers.
The original daemon binary and process are unchanged. Additional compositor Lua
memory was not independently benchmarked; no zero-memory or battery-savings
claim is made.

Private baseline hashes and screenshots/logs are under ignored `.agents/`.
