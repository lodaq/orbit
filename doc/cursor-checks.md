# Smooth cursor implementation and checks

Search anything keeps its native Slint TextInput. The block follows final input coordinates inside a clipped overlay aligned with the existing Flickable, including its vertical scroll offset. Settings → Search bar → General → Smooth cursor saves immediately; old configs default to On. No production dependency or per-frame Rust work was added.

## Files

- `ui/theme.slint`: cursor color, size, opacity and motion tokens, including the one-shot line-snap delay.
- `ui/controls.slint`: reusable SmoothCaret with fast/slow horizontal positions, bounded trail, selection fade and idle timer.
- `ui/launcher.slint`: search overlay, native-caret visibility events, selection/IME handling, edit reset and Settings control.
- `src/config.rs`: backward-compatible saved boolean and config regression test.
- `src/apps.rs`: existing worker-based setting save, native-caret event connection and UI synchronization.
- `src/search-cursor.rs`: safe event-driven native blink suppression using the pinned Slint internals.
- `src/shortcuts.rs`: configurable previous-word action, native selection/deletion and shortcut tests.
- `src/main.rs`: search cursor module registration.
- `Cargo.toml`: existing testing backend software-renderer feature and existing image crate PNG support for tests.
- `Cargo.lock`: testing renderer dependency edge.
- `tests/caret-geometry.rs`: geometry, retargeting, line snapping, inactive startup, selection and idle tests.
- `tests/caret-ui-check.rs`: reproducible software captures, active overlay disposal and actual search input checks.
- `src/platform.rs`: existing test module moved after production functions to satisfy all-target Clippy.
- `src/storage.rs`: existing test module moved after production functions to satisfy all-target Clippy.
- `doc/design-system.md`: cursor design tokens, behavior and Slint compatibility notes.
- `doc/spec.md`: current cursor behavior and saved setting.
- `doc/glossary.md`: approved Smooth cursor setting term.
- `doc/structure.md`: cursor test and capture locations.
- `doc/cursor-checks.md`: verification instructions and limitations.

## Slint APIs and adaptations

Slint and slint-build remain pinned to 1.18.1. Source checks used the installed `i-slint-compiler-1.18.1/builtin_elements.rs` TextInput and Timer declarations and `i-slint-core-1.18.1/items/text.rs` cursor, focus and external-text change handlers.

- `cursor-position-changed(Point)` supplies final input-relative coordinates; the existing callback continues scrolling the query.
- `cursor-position-byte-offset`, `anchor-position-byte-offset` and `preedit-text` are available but undocumented; regression-check these when upgrading Slint.
- The event-driven native cursor bridge sets `TextInput.cursor_visible` false, dropping the native blink binding while the block is active. Off or IME composition restores `WindowInner::set_cursor_blink_binding`. Both zero-width and subpixel-width suppression proved insufficient over a full FemtoVG blink cycle. This uses Slint 1.18.1 safe internal APIs; recheck them when upgrading.
- Font ascent minus descent supplies block height. Wrapped-line changes snap; only horizontal position stretches.
- Bound animation durations, property change callbacks and declarative Timer start/stop work in this version. The one-shot snap timer does not repeat after a line change.
- External text assignments already call `align_to_text`, clamp offsets and trigger the cursor callback; no replacement editor or Rust position estimation is needed. Programmatic clear is captured with the real Launcher.
- The installed mock-time backend fires declarative Slint timers even with `init_no_event_loop()`, so idle tests run alongside geometry tests on isolated test threads; no separately initialized process-wide event loop is required.
- Removing the conditional overlay drops its timer and animated properties. Tests verify no timer deadline remains and no animation is active on the next clock tick, including removal during movement.

## Commands and results

This environment needs `/home/sebaq/.cargo/bin` on PATH.

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test caret-ui-check
cargo build --release
cargo check --target x86_64-pc-windows-gnu
ORBIT_INPUT_SOCKET=/run/user/1000/.ydotool_socket ORBIT_POINTER_SCALE=2 python3 tests/desktop-gestures.py
```

Formatting, both Clippy commands and Rust tests pass: 55 unit tests plus 7 cursor tests and 1 capture/search test; 6 existing live-account/reproduction tests remain ignored. The capture test is included in `cargo test`. Release build and restart are required after source changes. Linux production uses FemtoVG; headless captures use the software renderer.

The stretch mutation check temporarily gave slow-x the lead duration. The movement test failed with `span 7`, then the original implementation was restored and the suite passed. Evidence: `/tmp/orbit-caret-mutation.log`.

The live hidden sample consumed 0 CPU ticks over 5.00 seconds (0.000% of one CPU). Post-search RSS was 222972 KiB (228 MB), above the existing 140 MB repository budget; memory attribution against an equivalent baseline search session was not measured.

The desktop gesture script passed `center` and failed `edge` because an overlapping drop created a folder. The same `edge` assertion fails on untouched HEAD `c70c283`; no gesture code was changed. Current evidence: `/tmp/orbit-gestures-_f5iqykj`; baseline evidence: `/tmp/orbit-gestures-8jz04y5v`. The remaining gesture scenarios were not reached.

Windows cross-check is blocked before Orbit compilation: the Rust target is installed but `ring` cannot find `x86_64-w64-mingw32-gcc`. Real Windows behavior is unverified. No fcitx5 or ibus session is running, so real IME composition is unverified; the overlay uses the native pre-edit state and preserves TextInput handling. Input remains synchronous in tests; end-to-end typing latency was not benchmarked against the baseline.

## Captures

All cursor images live under `/home/sebaq/repos/orbit/target/ui-check/`. `cargo test --test caret-ui-check` reproduces `cursor-rest.png`, `cursor-move-start.png`, `cursor-stretch.png`, `cursor-selection.png`, `cursor-unfocused.png`, `cursor-disabled.png`, `cursor-reopened.png`, `cursor-hide-start.png`, `cursor-hide-end.png`, and the real component captures `cursor-search-typed.png`, `cursor-search-selection.png`, `cursor-search-disabled.png`, `cursor-search-cleared.png`, `cursor-search-wrapped.png`. The isolated state gallery intentionally does not replace native editing.

`cursor-desktop-*.png` records the live Hyprland search and Settings paths. `cursor-report.txt` is the reproducible headless summary; `cursor-validation.log` contains the checks; `cursor-desktop-report.txt` records live checks and hidden CPU/RSS. Existing `report.txt` and earlier screenshots are retained.

## Native caret and previous-word follow-up

The original width-based suppression (0 px, then 0.01 px) did not eliminate Slint’s independently blinking caret. Earlier single-frame captures missed its on phase. Width suppression has been removed; `src/search-cursor.rs` now disables the native visibility/blink binding directly after input and focus changes. It does no per-frame work. Smooth cursor Off and IME composition restore the native binding.

A 32-frame live FemtoVG recording over several seconds caught the old white vertical line: its red channel changed from 41 to 185 at column 80 across 16 pixels. With visibility suppression, no corresponding high-contrast blink remained across 32 frames. Frames are in `target/ui-check/cursor-blink-word-before/` and `cursor-blink-fixed/`. A final 32-frame recording after live Ctrl+W also found no native white blink: `target/ui-check/cursor-blink-after-ctrl-w/`; summary: `target/ui-check/cursor-ctrl-w-desktop-report.txt`. The blue block continues its separate Slint opacity animation.

The rendered NativeCaretProbe verifies suppression across six 250 ms steps and verifies that native white pixels return when disabled. Settings → Shortcuts now lists Delete previous word, initially Ctrl+W, and supports existing Change/Delete controls. Existing configurations receive it once if Ctrl+W is unused. Removal remains permanent; existing conflicting bindings are preserved.

Previous-word deletion uses Slint’s native word selection and deletion methods, including selection, Unicode boundaries, undo and clipboard preservation. It does not synthesize or change modifier keys, and is scoped to focused Search anything with no Settings or layer open. Tests cover Latin/Arabic text, emoji preceding a word, undo, rebinding, disabled contexts, defaults and config migration.

Live screenshots: `target/ui-check/ctrl-w-word-deleted.png`, `ctrl-w-undo.png`, `ctrl-w-settings.png` and `ctrl-w-settings-edit.png`. Formatting, all-target Clippy, all 63 enabled tests and the release build pass. Check log: `target/ui-check/cursor-shortcut-validation.log`. The native caret bridge and shortcut were checked through real launcher and Settings entry points after a release restart.

## Search overflow and Kbd follow-up

Search input and text measurement now share a width that reserves the complete cursor block plus 2 px edge insets. The overlay includes the input offset; vertical scrolling uses the full font-metric cursor height and guarantees bottom clearance, including the last line at the 176 px maximum search height. The field uses 14 px text, 48 px minimum height, consistent padding, an aligned Prefix badge and a quiet focus border.

A shared native Slint Kbd control follows the shadcn/ui rounded muted keycap reference. It replaces search, footer, Settings close, binding, recorder and Shift + wheel shortcut text. Saved shortcut hints are supplied by the existing worker update, so changed or removed bindings are reflected. No dependencies were added.

The software-renderer test activates a Prefix through Tab, moves to the end of three overflow queries (including the reported example and a query much taller than the viewport), and asserts a blue block at least 7 px wide and 16 px high. Captures: `target/ui-check/cursor-prefix-overflow-0.png`, `cursor-prefix-overflow-1.png`, `cursor-prefix-overflow-2.png`.

The rebuilt release was restarted on Hyprland; checked the actual Search anything → t → Tab → long query path, Settings → Shortcuts → Change, and footer hints. Live captures: `target/ui-check/kbd-launcher.png`, `kbd-prefix-search.png`, `kbd-shortcuts.png`, `kbd-recorder.png`. Formatting, all-target Clippy and all 63 enabled tests pass; log: `target/ui-check/kbd-validation.log`.
