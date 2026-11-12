# File structure

- `src/ink.rs`: world-coordinate ink paths, bounded stroke input and whole-stroke erasing.
- `src/canvas-edit.rs`: bounded session history and Canvas item clipboard snapshots, fresh IDs and collision-free paste placement. `tests/canvas-edit-ui.py` checks scoped edit/tool shortcuts, wheel pan/zoom, Frame navigation, live ink and gesture cancellation on a private display.
- Canvas Frames reuse `src/config.rs` storage, `src/spatial.rs` selection and reconciliation, `src/apps.rs` worker saves and fit navigation, and `src/shortcuts.rs` recording and conflict checks. `ui/assets/tool-*.svg` supplies the four toolbar icons.

- `src/t3-providers.rs`: bounded local T3 Code provider/model import, secret-free account references and connection fingerprints.
- `src/robot-native.rs` and `src/robot-native/tests.rs`: direct Codex app-server, ACP and Pi RPC execution, permission routing, bounded output, credential redaction and cancellation fixtures.
- `tests/native-provider-fixture.py`: offline stdio agent for protocol and permission checks.

- `robot-capabilities.md`: authorized Robot capability boundaries, integration points, build order and release gates.
- `src/robot-data.rs`: versioned `robots.db`, validated Robot/Role profiles, immutable conversation context, ordered activity, ratings, deletion and transactional backup snapshots. The execution and workbench layers will use this storage API.
- `src/robot-data/tests.rs`: persistence, conversation snapshots, Unicode titles, history, deletion, validation and import rollback checks.
- `src/robot-tools.rs`: memory-only local approvals, bounded file/Skill reads, atomic text writes, directory listing, approved process execution and Robot keyring lookups.
- `src/robot-tools/mcp.rs`: bounded stdio JSON-RPC sessions, MCP initialization/discovery, enabled-tool checks and credential redaction.
- `src/robot-tools/web.rs`: provider-independent Brave Search with linked sources and a keyring credential.
- `src/robot-provider.rs`: native Ollama, OpenAI Responses and Gemini multi-turn requests, tool-call parsing, in-memory provider continuation state and resumed conversation context.
- `src/robot-runtime.rs`: one worker's persistent command queue, conversation-bound approvals, cancellation, Skill loading, automatic enabled MCP calls and saved activity, independent of launcher visibility. The workbench will own its single Handle and consume its events.
- `src/robot-provider/tests.rs` and `src/robot-runtime/tests.rs`: provider protocol fixtures, queue order, context snapshots, approval/cancellation, restart recovery, observer-independent completion and an isolated Skill/MCP/report workflow.
- `src/robot-tools/tests.rs` and `mcp-tests.rs`: approval, local side-effect, process timeout, MCP fixture and redaction checks; web source checks live beside the parser.
- `doc/adr/0002-robot-execution.md`: Robot storage, execution and permission decisions, with implementation status.

- `tasks/plan.md` and `tasks/todo.md`: the current English assistance repair plan, ordered tasks and acceptance checks.

- `src/main.rs`: the `daemon`, `toggle` and `ticktick` commands.
- `src/config.rs`: versioned flat Canvas config and saved Canvas speed, legacy layout migration, background color, image and image-folder settings, app, file and directory search paths, file-search exclusion defaults, named Actions and Prefixes, first-run data, and damaged config recovery.
- `src/add-item.rs`: Add item state for the cached Installed apps inventory, cancellation of stale requests, and local field validation.
- `src/apps.rs`: app and folder actions, Canvas placement, search across items, folder contents and configured paths with file exclusion matching and worker cache invalidation, generic Prefix recognition/dispatch, dock pinning and Recent display, background image selection, website favicon retrieval, and cached icons capped at the displayed size; file writes and decoding run on a worker thread.
- `src/recent.rs`: saved Recent order, duplicate removal, and the history limit.
- `src/daemon.rs`: the resident Slint event loop and toggle delivery.
- `src/platform.rs`: config and Recent file access and atomic writes, installed app discovery including configured executables and executable symlinks, detached commands, the Unix socket, single-daemon lock, Hyprland monitors, native file chooser rules applied before mapping, fullscreen, native Wayland touchpad pinch input, and window hiding.
- `ui/assets/`: vector artwork extracted from `doc/orbit-ui/orbit.svg`, including the Home Screen image-change icon. `search-engines/` contains bundled search engine brand icons and their source list.
- `ui/fonts/`: embedded Inter font and its OFL license.
- `ui/launcher.slint`: the top bar, banner, Prefix badge and search results, full-window Canvas, fixed controls, Home Screen Settings with Canvas speed, Appearance Settings, Search bar Settings with General, Prefixes, Actions and Scripts, TickTick Widget Gallery and task editor, app and folder sheets, open folder panel, Icon picker, item and empty-space context menus, and delete alert; `build.rs` compiles it.
- `Cargo.toml` and `Cargo.lock`: dependencies and their locked versions. Speed results are in `doc/spec.md`, section 3.

The single Canvas amendment replaces page management and swipes with free item placement, pan and zoom. Step 5 adds Websites, Files, Directories, Actions, Custom command and Installed apps. Pinning and Recent come with the dock in step 6, grid shortcuts in step 7, Settings and appearance in step 8, glyphs in step 9, and packaging in step 10.

The approved supplied UI update adds the reference launcher appearance, search across saved Canvas items, a pinned dock, and a Settings panel. A later request adds Recent to the dock. Search path management extends Settings with saved file and directory roots. These are explicit exceptions to the original step order, not completion of the remaining steps.

- `ui/theme.slint`: shared colors and design tokens for spacing, type, sizes, radii and borders.
- `ui/controls.slint`: shared buttons, result rows and labeled fields with scrollable multiline editing, plus Settings cards, navigation, tabs, switches, selectors, steppers and removable chips built from the design tokens.
- `doc/design-system.md`: shared appearance and interaction rules.
- `doc/widgets.md`: widget purpose, current TickTick support, widget behavior, and future widget proposals.

- `src/icons.rs`: icon drafts, visible-character validation, worker decoding and native icon chooser callbacks.

- `src/backup.rs`: the platform module’s backup export, validation and import, including the native chooser and retention of previous data.

- `src/shortcuts.rs`: saved shortcut defaults, conflict validation, key recording, layout-independent modified letters and action routing.
- Search Actions in `src/config.rs` include Browser Search (`s`), which opens URL-like input directly and searches other text with Brave, and account-scoped GitHub repositories (`repo`). `ui/controls.slint` shares list-focus and modifier state; focused lists use Ctrl+N/J/Down and Ctrl+P/K/Up.

- `src/item-drop.rs`: shared item move rules for Canvas and Folder drops, stable ID lookup, shortcut-preserving folder extraction, empty folder cleanup, and regression tests.

- `src/spatial.rs`: signed rectangular Canvas positions, deterministic free-space placement, fixed-neighbor rectangle placement, widget footprints, and spatial regression tests.
- `src/ticktick.rs`: TickTick API requests, personal-token storage in the operating system credential store, and task moves, which the API only accepts as a copy into the new list followed by deleting the original.
- `src/toast.rs`: user feedback levels, short message queue, duplicate suppression and handling while the Launcher is hidden.
- `src/storage.rs`: shared versioned config, recent, installed-app, icon and backup storage across supported desktop platforms; bounded Website icon fetching, ranked declarations and HTTP connection reuse.
- `src/widget-data.rs`: the local SQLite text collections, stable entry IDs, editing, deletion, search, pagination and storage sizes.
- `src/widget-data-ui.rs`: Settings → Data callbacks, collection counts, storage meter values, completed TickTick titles and a dedicated worker; database initialization, migrations and reads stay outside the launcher show path.
- `src/platform-windows.rs`: Windows resident launcher IPC, fullscreen window behavior, registered-file opening, process launch, system actions and Installed apps discovery.
- `tests/canvas-groups-ui.py`: private-display selection, Frame creation/editing, shortcut conflicts, layout lock, mixed-language labels and visible ink persistence/erasing checks.
- `tests/canvas-ui.py`: real pointer and persistence checks using a private Xvfb display, temporary config and isolated runtime socket.
- `tests/dock-toggle-ui.py`: dock-revealed special workspace closure and repeated toggles using a private display/profile and simulated Hyprland workspaces.
- `tests/search-state-ui.py`: private-display checks for search text, cursor and Prefix retention across toggles and focus interruptions, plus explicit Escape clearing.
- `tests/scroll-pan-ui.py`: continuous wheel pans across Widgets, normal new Widget scrolls, unchanged item positions and view persistence on a private display/profile.

The approved v2 UX specification is `Orbit_Desktop_Launcher_UX_Spec_v2.md`; it supersedes the original OS target and list-grid assumptions.

- `src/ticktick-ui.rs`: TickTick request coordination, optimistic snapshots and rollback, dynamic list models, calendar views and result feedback.
- `ui/assets/task-complete.wav`: short completion sound played by the desktop platform module.

- `tests/caret-geometry.rs`: mocked-time cursor geometry, line snapping, selection and idle regression checks.
- `tests/caret-ui-check.rs`: reproducible software-renderer captures and real search input checks; `cargo test --test caret-ui-check` writes `target/ui-check/cursor-*.png` and `cursor-report.txt`.
- `doc/cursor-checks.md`: cursor API compatibility, validation commands, captures and known verification limits.
- `src/search-cursor.rs`: event-driven bridge to Slint 1.18.1 native caret visibility; preserves native editing and IME without per-frame work.
- Shared `Kbd` labels in `ui/controls.slint` render shortcut hints, Settings bindings and recorder previews. `tests/caret-ui-check.rs` also checks complete cursor bounds at long Prefix query ends.

- `ui/assets/lock.svg` and `ui/assets/unlock.svg`: top-bar layout lock states. `src/config.rs` persists the lock; `src/apps.rs` guards layout actions and synchronizes it with `ui/launcher.slint`.

- `ui/assets/add-*.svg`: embedded type and System action line icons from the Add item reference, with matching media controls. `AddItemRow` in `ui/controls.slint` presents cached app details and explicit Add controls.

- `src/website-icons.rs`: debounced Website icon preview worker, stale-result checks, in-memory fetched drafts and custom/automatic icon selection for Save.

- `doc/english-assistance.md`: approved local Search anything suggestions, autocomplete and grammar feedback, including settings, staged behavior, verification results and measured limits.

- `src/english-assistance.rs`: local Ollama model discovery and settings, editable prompts, background model requests, latest-text cancellation, source-aware completion edits, response validation and assistance results.
- `src/search-edit.rs`: native selection replacements and grouped undo/redo for assistance edits.

- `src/ai.rs`: independent AI provider/model settings, local model discovery, credential-store access, single-question HTTP requests, response validation and cancelled/stale-result protection.
- Search bar → AI and the AI Prefix answer panel in `ui/launcher.slint` reuse shared controls and native text selection/clipboard behavior.

- `src/widget-starters.rs`: original bilingual starter text, vocabulary examples, unfinished starter habits, Quran passage attribution and a dated Cairo prayer-time example.
- `src/widget-views.rs`: local selections, persistent shuffled rotations, completion references, habit state, today's achievements and saved prayer countdowns.
- `src/internet-usage.rs`: worker-side vnStat JSON reading, local-day interface totals, binary unit formatting and last-successful-value caching.
- `src/internet-limits.rs`: validated kbps conversion and atomic Linux nftables limits for this computer, with active kernel-state reading.
- `ui/internet-limits.slint`: shared Internet Usage speed-limit sheet and its draft/loading state.
- `tests/internet-limits-ui.py`: real widget controls and IPv4/IPv6 transfers on private display/profile/network namespaces.
- `ui/local-widget.slint`: local widget reading hierarchy, scrollable content, fixed action footers, grouped habit rows, an Achievements summary with scrollable completion rows, and local audio actions, displayed above the Canvas input surface. `src/widget-data-ui.rs` separates vocabulary and prayer headlines for display without changing saved content.

- `Panel`, `MenuOption` and `SettingsToggle` in `ui/controls.slint` share floating material, menu rows and labeled switches across the complete native interface.

Search locations in `ui/launcher.slint` groups all search paths and file exclusions; Refresh in `src/apps.rs` rebuilds both Installed apps and the file/directory cache on the worker.

Search-result saving in `src/apps.rs` resolves cached results to shortcuts; `src/item-drop.rs` validates atomic Canvas placement, existing-item extraction and exclusive dock ownership. `ui/launcher.slint` provides result context actions and a search drag mode that reveals the Canvas while preserving the source result.

- `src/adhkar.rs`: combined Remembrances prayer-time selection, legacy pair progress migration, Morning and Evening starter import, per-mode SQLite progress, repetition counting, navigation, completion rules and daily Achievements recording.
- `data/morning-adhkar.json`: the 25 Arabic remembrances supplied by the user, preserved with their source details.
- `data/evening-adhkar.json`: the 25 supplied evening remembrances, preserving their wording, repetition counts, source details and notes.
- `ui/fonts/amiri.ttf` and `AMIRI-OFL.txt`: embedded Amiri Arabic reading font and its SIL OFL license.
- `ui/assets/adhkar-complete.wav` and `adhkar-all-complete.wav`: distinct generated chimes for one remembrance and the full collection.

- `src/azan.rs`: per-widget prayer locations, named time-zone validation, explicit Aladhan schedule requests, accepted shared schedule references (including sunrise) and zone-aware countdowns.
- `ui/azan-settings.slint`: Azan settings state and the location/preview sheet; the Azan reading card remains in `ui/local-widget.slint`.

`ui/controls.slint` owns `OrbitText` and the `TextDirection` callbacks for content-aware text alignment throughout the interface.
- `src/text-direction.rs`: shared Unicode first-strong direction detection for Arabic, English and mixed text.

- `src/canvas.rs`: saved Canvas view, signed coordinates, pointer-centered zoom, fit-all recovery, scroll sequence ownership shared by the platform event filters, and Slint callbacks using the existing config worker.
- `ui/canvas.slint`: shared Canvas view state and pan/zoom callbacks; launcher rendering remains in ui/launcher.slint.

The 2026-11-02 single Canvas amendment supersedes historical page-related structure descriptions above. Config version 5 stores a flat item collection, signed positions and view state; legacy pages exist only for migration. There are no live pages or Boxes.
- `doc/adr/0001-single-canvas.md`: the Canvas model and original-config migration backup decision.

Robot workbench integration:
- `tests/robot-management-ui.py`: private-display, temporary-profile checks for chat replies, shell approval, repeated actions, management collections, Robot name propagation, restart persistence and compact layouts.
- Robot database schema 4 stores validated shared collections and includes them in backup snapshots. `ui/robot-workbench.slint` separates eight-section management from Canvas chat; `src/robot-workbench.rs` handles explicit Skill/provider refresh, collection changes, repeated commands and profile duplication on the worker.
- `src/robot-workbench.rs`: worker-side profile/Role/tool management, conversation navigation, exact approval routing and runtime event delivery. Import suspends the idle runtime before the data directory is replaced.
- `ui/robot-workbench.slint`: shared Robot settings and workbench layer, including approval review, conversation history, ratings and permanent deletion.
- `ui/assets/robot.svg`: one shared 2D Robot illustration for Canvas items and entry points.

Robot provider accounts:
- `robot-provider-capabilities.md`: approved account, connection and native execution extension and acceptance gates.
- `src/robot-data.rs`: named account metadata, validation, schema-3 persistence and immutable per-conversation connection snapshots.
- `src/robot-workbench.rs` and `ui/robot-workbench.slint`: account selection/editing in Robot profiles, using the shared fields and controls.
- `src/robot-provider.rs` and `src/robot-runtime.rs`: account credential resolution, import reconnection checks and explicit errors for unconnected native providers.
- `src/backup.rs`: version-4 account metadata export/import, without provider credential values or account-directory contents.

- `src/scripts.rs`: Script query parsing, file/name validation and alphabetized UI models. Config and backups retain Script entries; `src/apps.rs` uses the existing worker for edits, native choosing and direct launches. `ui/launcher.slint` provides Search bar → Scripts and the `/` list. `tests/scripts-ui.py` exercises the real Settings and launch paths, native chooser, failure feedback, persistence, long lists and compact layouts on a private display/profile/bus.

Canvas drawing tools extend `src/ink.rs` with stroke styles, quadratic smoothing, arrows and eraser hit detection. Canvas notes, text and image assets use the existing Item storage, placement, history and backup paths. `ui/theme.slint` scales interface typography using the General Font size setting. `tests/canvas-tools-ui.py` exercises the new tools on a private display/profile/bus.

Canvas Images resize like Widgets from the handle at their corner and keep their own proportions. `src/apps.rs` holds the saved size and its limits; `ui/launcher.slint` draws the handle and the live size. `tests/image-resize-ui.py` imports an Image, resizes it with the handle and checks the result on a private display/profile/bus.
