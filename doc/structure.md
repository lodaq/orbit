# File structure

- `src/main.rs`: the `daemon` and `toggle` commands.
- `src/config.rs`: versioned config data, page limits, file and directory search paths, file-search exclusion defaults, named Actions and Prefixes, first-run data, and damaged config recovery.
- `src/add-item.rs`: Add item state for the cached Installed apps inventory, cancellation of stale requests, and local field validation.
- `src/apps.rs`: app and folder actions, page selection, page creation and deletion, search across pages and configured paths with file exclusion matching and worker cache invalidation, generic Prefix recognition/dispatch, dock pinning and Recent display, website favicon retrieval, and cached icons capped at the displayed size; file writes and decoding run on a worker thread.
- `src/recent.rs`: saved Recent order, duplicate removal, and the history limit.
- `src/daemon.rs`: the resident Slint event loop and toggle delivery.
- `src/platform.rs`: config and Recent file access and atomic writes, detached commands, the Unix socket, single-daemon lock, Hyprland monitors, fullscreen, and window hiding.
- `ui/assets/`: vector artwork extracted from `doc/orbit-ui/orbit.svg`. `search-engines/` contains bundled search engine brand icons and their source list.
- `ui/fonts/`: embedded Inter font and its OFL license.
- `ui/launcher.slint`: the top bar, banner, Prefix badge and search results, scrolling page grid, page dots, Home Screen Settings, Search bar Settings with General, Prefixes and Actions, TickTick Widget Gallery and task editor, app and folder sheets, open folder panel, Icon picker, item and empty-space context menus, and delete alert; `build.rs` compiles it.
- `Cargo.toml` and `Cargo.lock`: dependencies and their locked versions. Speed results are in `doc/spec.md`, section 3.

Steps 1–4 are complete. Step 4 adds folders, item drops, page swipes and page creation. Leaving an empty page does not remove it. Step 5 adds Websites, Files, Directories, Actions, Custom command and Installed apps. Pinning and Recent come with the dock in step 6, grid shortcuts in step 7, Settings and appearance in step 8, glyphs in step 9, and packaging in step 10.

The approved supplied UI update adds the reference launcher appearance, search across saved pages, a pinned dock, and a Settings panel. A later request adds Recent to the dock. Search path management extends Settings with saved file and directory roots. These are explicit exceptions to the original step order, not completion of the remaining steps.

- `ui/theme.slint`: shared colors and design tokens for spacing, type, sizes, radii and borders.
- `ui/controls.slint`: shared buttons, result rows and labeled fields, plus Settings cards, navigation, tabs, switches, selectors, steppers and removable chips built from the design tokens.
- `doc/design-system.md`: shared appearance and interaction rules.

- `src/icons.rs`: icon drafts, visible-character validation, worker decoding and native icon chooser callbacks.

- `src/backup.rs`: the platform module’s backup export, validation and import, including the native chooser and retention of previous data.

- `src/shortcuts.rs`: saved shortcut defaults, conflict validation, key recording and action routing.

- `src/item-drop.rs`: shared item move rules for page and folder drops, stable ID lookup, shortcut-preserving folder extraction, empty folder cleanup, and regression tests.

- `src/spatial.rs`: explicit rectangular grid positions, deterministic reflow, fixed-neighbor rectangle placement, widget footprints and legacy layout migration, and spatial regression tests.
- `src/ticktick.rs`: TickTick API requests, personal-token storage in the operating system credential store, and task moves, which the API only accepts as a copy into the new list followed by deleting the original.
- `src/toast.rs`: user feedback levels, short message queue, duplicate suppression and handling while the Launcher is hidden.
- `src/storage.rs`: shared versioned config, recent, installed-app, icon and backup storage across supported desktop platforms.
- `src/platform-windows.rs`: Windows resident launcher IPC, fullscreen window behavior, registered-file opening, process launch, system actions and Installed apps discovery.
- `tests/desktop-gestures.py`: opt-in real pointer and persistence checks using an isolated config directory on Hyprland.

The approved v2 UX specification is `Orbit_Desktop_Launcher_UX_Spec_v2.md`; it supersedes the original OS target and list-grid assumptions.

- `src/ticktick-ui.rs`: TickTick request coordination, optimistic snapshots and rollback, dynamic list models, calendar views and result feedback.
- `ui/assets/task-complete.wav`: short completion sound played by the desktop platform module.

- `tests/caret-geometry.rs`: mocked-time cursor geometry, line snapping, selection and idle regression checks.
- `tests/caret-ui-check.rs`: reproducible software-renderer captures and real search input checks; `cargo test --test caret-ui-check` writes `target/ui-check/cursor-*.png` and `cursor-report.txt`.
- `doc/cursor-checks.md`: cursor API compatibility, validation commands, captures and known verification limits.
- `src/search-cursor.rs`: event-driven bridge to Slint 1.18.1 native caret visibility; preserves native editing and IME without per-frame work.
- Shared `Kbd` labels in `ui/controls.slint` render shortcut hints, Settings bindings and recorder previews. `tests/caret-ui-check.rs` also checks complete cursor bounds at long Prefix query ends.

- `ui/assets/lock.svg` and `ui/assets/unlock.svg`: top-bar layout lock states. `src/config.rs` persists the lock; `src/apps.rs` guards layout actions and synchronizes it with `ui/launcher.slint`.
