# File structure

- `src/main.rs`: the `daemon` and `toggle` commands.
- `src/config.rs`: versioned config data, page limits, file and directory search paths, first-run data, and damaged config recovery.
- `src/add_item.rs`: Add item state for the cached Installed apps inventory, cancellation of stale requests, and local field validation.
- `src/apps.rs`: app and folder actions, page selection, page creation and deletion, item drops, search across pages and configured paths, dock pinning and Recent display, website favicon retrieval, and cached icons capped at the displayed size; file writes and decoding run on a worker thread.
- `src/recent.rs`: saved Recent order, duplicate removal, and the history limit.
- `src/daemon.rs`: the resident Slint event loop and toggle delivery.
- `src/platform.rs`: config and Recent file access and atomic writes, detached commands, the Unix socket, single-daemon lock, Hyprland monitors, fullscreen, and window hiding.
- `ui/assets/`: vector artwork extracted from `doc/orbit-ui/orbit.svg`.
- `ui/fonts/`: embedded Inter font and its OFL license.
- `ui/launcher.slint`: the top bar, banner, search results, scrolling page grid, page dots, Settings path lists, app and folder sheets, open folder panel, Icon picker, context menu, and delete alert; `build.rs` compiles it.
- `Cargo.toml` and `Cargo.lock`: dependencies and their locked versions. Speed results are in `doc/SPEC.md`, section 3.

Steps 1–4 are complete. Step 4 adds folders, item drops, page swipes, page creation, and removal of an empty page when leaving it. Step 5 adds Websites, Files, Directories, Actions, Custom command and Installed apps. Pinning and Recent come with the dock in step 6, grid shortcuts in step 7, Settings and appearance in step 8, glyphs in step 9, and packaging in step 10.

The approved supplied UI update adds the reference launcher appearance, search across saved pages, a pinned dock, and a Settings panel. A later request adds Recent to the dock. Search path management extends Settings with saved file and directory roots. These are explicit exceptions to the original step order, not completion of the remaining steps.

- `ui/theme.slint`: shared colors from the Add item reference.
- `ui/controls.slint`: shared buttons, result rows and labeled fields.
- `doc/DESIGN_SYSTEM.md`: shared appearance and interaction rules.

- `src/icons.rs`: icon drafts, visible-character validation, worker decoding and native icon chooser callbacks.
