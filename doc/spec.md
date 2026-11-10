# V2 authority

The user approved `Orbit_Desktop_Launcher_UX_Spec_v2.md` on 2026-10-20 as authoritative wherever it conflicts with this document. Windows and Arch Linux, spatial grids, widgets, exclusive dock ownership, new UI terms, and the complete v2 build sequence are authorized. Existing visual artwork and nonconflicting performance and coding rules remain in force. The older build-step restriction does not apply to v2.

# Orbit spec

## Robot management and chat (authorized 2026-11-05)

The combined Settings → Robots entry replaces the separate Providers and Robots entries. Its navigation contains Robots, Skills, Providers, Actions, MCPs, Credentials, Projects and Roles. Management uses a compact dark sidebar and content area; Canvas Robot clicks open a separate chat view with conversations, messages, saved Actions and a bottom composer. Profile editing is reached through management or the Canvas Context menu, never through the chat header.

Custom Skills register existing SKILL.md files. Refresh from ~/.agents explicitly rereads ~/.agents/skills on the worker. Registered Projects select existing absolute directories. Actions save reusable prompts or shell-command instructions executed by the Robot through its existing tools and permission flow. Credentials list references only; values remain masked and live only in the system credential store. Saved MCP definitions are reusable; attachments remain per Robot and require saving its profile. Existing native-tool limitations continue to apply.

Providers support manual accounts and explicit Fetch from T3 Code. Opening management or chat does not refresh T3 configuration. Removing an account or Role still assigned to saved data is rejected. Removing a Robot profile and its history requires confirmation and removing its Canvas references first. Canvas Robot Context menus include Edit, Pin/Unpin, Delete, Duplicate and the three most-used saved conversation commands, ranked by frequency then recency. Repeating a command uses the existing queue and approval flow. Duplicating creates an independent profile and places a new Canvas item without moving its neighbors.

Robot database schema 4 adds collections for Skills, Actions, Projects, MCP definitions and credential references. Backup version 5 includes these collections in validated, transactional restore; credential values and Skill/project directory contents are excluded. Imported MCP definitions require their tools to be enabled again.

Verification (2026-11-06): the release launcher passed the real Canvas → chat/actions and Settings → Robots management paths on a private Xvfb display and temporary profile. Checks covered prompt replies, exact shell approval/output, repeated commands, independent duplication, explicit Skill/provider refresh, collection add/remove, Robot create/edit/remove, Canvas name updates without moving items, restart persistence, and 1280×800 chat/management with reachable MCP Save. Captures: `target/ui-check/canvas-ja7kvrrn/`. Formatting, Clippy, the release build and 172 Rust tests passed; six existing live-account tests were ignored. Live credentials/external MCP services and performance budgets were not rechecked.

## Shared Providers (authorized 2026-11-05)

Settings → Robots → Providers lists shared provider accounts. It lists T3 Code's configured instances, including disabled instances, and their locally cached status and models. Refresh from T3 Code rereads local configuration on the worker; showing the launcher does no provider discovery or network work. Each Robot profile selects a shared provider account and model. Existing manual accounts remain available.

The user authorized completing connection and execution together for this request. Orbit reads T3 Code configuration and existing authentication references without changing its settings or copying authentication files. Only account metadata, source-directory references and a connection fingerprint enter robots.db and backups. Changing a T3 connection stops an existing conversation instead of silently switching its account. Imported references require local reconnection. Cached T3 status is not a live authentication check.

Codex uses app-server; OpenCode, Antigravity and installed registry agents use ACP; Pi uses RPC. Configured executables launch directly without the T3 Code application running. Missing executables, unavailable authentication and unsupported protocols fail explicitly without fallback or automatic sign-in.

Before launching a native agent, the workbench asks for command-scoped permission: native tools may read/change files, execute programs, access the network and use their configured MCP servers. This is explicitly broader than an Orbit local-tool approval. Subsequent native permission requests belong to the owning conversation and accept only single-use options. Pi tools use the displayed command-scoped permission. Cancellation terminates the provider process group; completed effects cannot be rolled back. Native agents retain their own configured MCP tools. Orbit profile MCP attachments with individual enabled-tool lists fail explicitly until a native allow-list bridge is available; their permissions are never silently broadened.

## Script launcher (authorized 2026-11-10)

Settings → Search bar → Scripts saves a separate list of named files in config.json, included in Export data and Import data. Names contain no whitespace; duplicate names are allowed. New script clears the editor; Edit loads an entry; Delete removes only the entry. Name and Path are editable, Choose file uses the native chooser, and Save script rejects empty names or paths that are not existing files. Missing files in restored data remain editable.

A leading `/` in Search anything opens an alphabetized vertical Scripts list with names and paths. Text before the first space filters names case-insensitively; the remaining text is passed unchanged as one argument. With no space, no argument is passed. Up/Down select without wrapping; typing arguments keeps the selected script. Tab fills the selected name and preserves arguments. Enter completes a partial name first, then runs on the next press; complete names or queries with arguments run immediately. Clicking runs the selected file, and Escape clears the search. No match does nothing. Scripts take priority over Prefixes; newly saved Prefix Tokens cannot start with `/`. Search engines and English assistance are hidden in script mode.

Files run directly and detached, with their folder as the working directory. Orbit neither invokes a shell nor guesses an interpreter or changes permissions. Linux scripts need an executable file and a valid shebang. A successful process start hides Orbit; a start failure leaves it open with a specific Toast. Windows accepts directly executable .exe files and reports an explicit unsupported-type error otherwise. This does not capture output or detect errors after a process has started.

Verification (2026-11-10): formatting, Clippy, release build and 194 Rust tests passed; eight existing opt-in tests remain ignored. `tests/scripts-ui.py` passed the real Settings and search paths on a private Xvfb display/profile/bus: native chooser with focus preserved, invalid-path feedback, add/edit/delete, stable IDs and restart persistence, case-insensitive filtering, no matches, keyboard/mouse launches, arguments preserving selection and spacing, no-argument launch, permission error Toast without hiding, Escape, duplicate names, scrolling to the final Arabic row, and the reachable editor at 1280×800. Config/backup round trips and direct execution without shell fallback passed. Captures: `target/ui-check/canvas-0gp30i_4/`. Windows runtime and performance budgets were not rechecked.

Completion verification (2026-11-10): formatting, Clippy, release build, 194 enabled Rust tests and the private-display Script UI check passed. Tab fills the selected name without launching; Enter completes a partial name before running on the next press. Completion preserves arguments and the selected file among duplicate names. Trailing-space completion, repeated Tab without extra spaces, and preserving existing arguments also passed; captures: `target/ui-check/canvas-b1jdm7x5/`. Windows runtime and performance budgets were not rechecked.

## Current behavior

### Canvas Groups and tools (authorized 2026-11-09)

The 2026-11-10 shortcut amendment adds session-only Undo/Redo for Canvas content edits, with up to 20 previous states, and Cut/Copy/Paste for selected Canvas items. Paste preserves relative positions, finds free space and gives copied items fresh IDs; the first paste of cut items preserves their IDs. The clipboard and history clear on restart or import. View changes are not Undo steps. Lock layout blocks edits and history restoration, while Copy remains available. These actions also appear in Canvas Context menus. Icon files remain available because clipboard items and Undo states can still reference them.

Wheel pans vertically; Shift+wheel pans horizontally; Ctrl+wheel zooms around the pointer. Widget content retains its own normal scrolling. With Canvas focus, Ctrl+J/L fits the next Group and Ctrl+H/K fits the previous Group, in saved order without wrapping. The existing Ctrl+K binding continues to focus Search anything outside the Canvas. Keys 1–4 select Select, Hand, Draw and Eraser. Canvas edit and tool bindings are managed in Settings → Shortcuts; migration preserves conflicting saved bindings. Text fields retain native typing, clipboard and history behavior. Draw and Eraser previews update during the held gesture, commit once on release, and Escape cancels the preview.

Verification (2026-11-10): formatting, Clippy, release build and 190 Rust tests passed; eight opt-in account tests remain ignored. The private-display Canvas edit check passed wheel pan/zoom, all Group/tool/edit shortcuts, live held Draw/Eraser previews, Escape and focus-loss cancellation, layout lock, mouse edit menus, native text editing and restart persistence. Captures: `target/ui-check/canvas-28yzl6mw/`. The existing Group creation/editing check also passed (`target/ui-check/canvas-wv6qun7g/`). Windows and performance budgets were not rechecked.

The left-edge toolbar provides Select, Hand, Draw and Eraser. Select drags a rectangle over Canvas items; Ctrl-click or Shift-click toggles individual items, and a modified rectangle adds to the selection. Hand pans the Canvas. Draw saves freehand ink in world coordinates; Eraser removes whole strokes. Lock layout prevents changes to Groups and ink while allowing navigation.

Right-click with selected items → New Group saves a unique name and optional shortcut. Items remain independently launchable at their existing positions. A labeled boundary follows the members. Clicking the label or pressing its shortcut fits the members using the existing Canvas fit behavior. Right-click the label to edit its name or shortcut, or delete the Group without deleting its items. Group shortcuts require Ctrl, Alt or Super and cannot duplicate saved action or Group shortcuts. Groups, ink and the view survive restart and are included in the existing config backup.

Verification (2026-11-09): `tests/canvas-groups-ui.py` passed on a private Xvfb display and temporary profile: rectangle and Ctrl-click selection, create/edit/delete, shortcut fit and conflict rejection, mixed Arabic/English labels and editor, layout lock, view/Group/ink persistence, visible ink after fitting and at negative coordinates, and erasing. Captures: `target/ui-check/canvas-85qy21nr/`. Formatting, Clippy, release build and 187 Rust tests passed; eight opt-in account tests were ignored. The older Canvas UI script passed navigation, pan, zoom, persistence and Add item entry, then stopped because its Custom command field coordinates predate the Robot row. Windows and performance budgets were not rechecked.

The single Canvas amendment approved on 2026-11-02 supersedes all earlier page and Box behavior, including the v2 layout specification. Canvas is the launcher's only arrangement space. There are no horizontal or vertical pages, swipe navigation, transitions, page indicators, page management or Boxes.

The Canvas fills the window behind the fixed logo, default search engine selector, Search anything, Lock layout, Settings, Close, Dock and Change background image controls. Their existing sizes and positions remain. The Canvas uses the existing blue-gray surface color at 50% opacity over the wallpaper; foreground controls and text retain their opacity. The wallpaper retains its previously approved 90% opacity. Review outlines are removed from the logo, engine selector, search, dock and icon buttons. Keyboard focus stays visible.

All existing launcher item types and thirteen supported widget types use the Canvas. Right-click empty space → Add item opens the existing sheet and widget gallery. New items use the clicked world position or a deterministic nearby free position; entry points without a pointer use the visible center. Auto places a widget near that anchor, and Place shows a movable placement preview. Additions, duplication, folder extraction and unpinning share free-space placement without displacing neighbors. Bring new content into view when necessary.

Positions are free, without grid snapping, and item rectangles must not overlap. Dragging and resizing show an invalid preview for occupied space; releasing restores the original position without saving. A completed 500 ms shortcut hold can create or fill a Folder. Use Hand or wheel to pan; Ctrl+wheel zooms around the pointer from 25% to 200%. Canvas speed in Settings → General controls both pan and zoom sensitivity from 1× to 10×, defaults to 5×, and Reset to default restores 5×. Widget content consumes scrolling normally. Pan and zoom persist across hiding and restart, with debounced saves. Escape cancels a drag or closes the top layer. Pending moves retain their dropped position until saved data is published; failed saves restore the saved position.

The empty-space Context menu also offers Show all items. It fits all Canvas rectangles, may zoom below 25%, and leaves an empty Canvas view unchanged. Zooming in from a fitted view must remain smooth. The left-edge toolbar provides the four authorized Canvas tools; no zoom label is shown. Lock layout prevents arrangement changes while permitting pan, zoom, Show all items, launching and widget interactions. Search results, Prefixes, English assistance and AI keep their existing presentation and behavior; saved-result placement is called Add to Canvas.

Config version 5 stores a flat item collection, signed Canvas positions and the saved view. Legacy configurations and backups migrate all horizontal and fixed-page layouts into adjacent regions without retaining page boundaries. Existing item IDs, dimensions, folder contents, dock ownership, Recent and widget state remain intact. Existing per-page widget migrations run before flattening. The first view starts at the formerly active region. The original configuration is retained in a timestamped backup before the migrated config is saved atomically. Boxes are removed from active storage and remain recoverable only from that original backup. A failed migration never replaces the original config or substitutes an empty saved configuration. Reopening does not repeat migration offsets. Legacy page structures exist only in decoding/migration paths.

Remembrances combines morning and evening Adhkar in one widget. It follows the first Azan Reminder's saved schedule and time zone: Fajr → Dhuhr selects morning, Dhuhr → next Fajr selects evening. Recommended windows are Fajr → Sunrise and Asr → Maghrib. Mode progress stays separate; evening continues through midnight. The gallery offers one Remembrances card; existing morning/evening pairs on each page combine without discarding saved progress. Prayer times come from the existing explicit Azan Fetch; no additional network requests run. See `widgets.md` for migration and missing/stale schedule behavior.

Settings → Data follows the supplied Data design: a storage meter with actual local sizes, grouped collection navigation with total counts, a scrollable entry list with attribution below content, and a selected-row editor beside the list. Narrow layouts stack the editor below the list. Search submits with Enter. Add entry and Edit preserve the full content and display character counts; errors appear inside the editor. Delete requires confirmation in the selected row. Pagination stays below the list. Achievements history is read-only.

Settings → Data uses one SQLite file, `widgets.db`, with separate tables for Quotes, Facts, Motivation, Affirmations, Vocabulary, Quran Player, Azan Reminder, Habits and Achievements. Version 1 content migrates transactionally with its row IDs preserved. Original starter content is added once. All nine local cards appear in Add item → Widgets with Auto and Place. Content rotates locally and daily selections survive restart; vocabulary advances on launcher opening. Habits, affirmations and learned words update achievements with typed finished IDs, and completed TickTick IDs are recorded under their completion dates, with task titles retained for Achievements labels. Achievements shows today with a completion count, date and scrollable checked rows; Data holds the history. Quran audio is imported into Orbit's audio folder and opens in the default audio application. Azan displays the latest saved schedule and warns if stale. API candidate Fetch, desktop reminders and embedded audio controls remain planned. Existing launcher backups do not include the widget database.

`orbit ticktick` shows the launcher with an empty query and the TickTick Prefix active, including when the launcher is already visible. The personal Hyprland Alt+T binding runs this command for quick task entry.

Search results show the two immediate parent directories of a file or directory, or the URL or custom command, beneath the name and source. Hovering a result or selecting it with arrow keys shows the full location in a wider detail strip above the dock.

Multiline fields scroll vertically when their content exceeds the visible area, without a scrollbar, and automatically keep the text cursor visible. This applies to task notes and descriptions, commands, SVG markup, Action descriptions and scripts, and model prompts.

Single-line fields scroll horizontally when their text exceeds the visible width, without a scrollbar, and automatically keep the text cursor visible. This includes task titles, names, URLs, paths, filters and credential fields.

Search anything excludes Folder tiles and searches their contained Apps, Websites, Files and Directories by name. Matching folder items open directly from search results.

Settings → Search bar → General includes App search paths. Each path is an absolute directory or executable. Refresh adds executable files immediately inside configured directories (no recursion), plus explicitly configured executable files, to the cached Installed apps list. Paths persist in config.json; changing them does not scan until Refresh. On Linux, executable symlinks retain their link name and launch path. Missing executable files, broken symlinks and non-executable files are omitted on refresh. Existing desktop discovery remains enabled. Refresh skips desktop entries whose executable is missing or not executable; command names are resolved through PATH.

Overlapping folder targets accept Apps, Websites, Files and Directories. After a 500 ms tile-overlap dwell, release creates a folder or appends to an existing folder. Existing Canvas items remain fixed during dragging and dropping. Occupied targets that cannot form a folder reject placement instead of displacing neighbors; empty-space drops move only the dragged item. Folder children retain shortcut type, URL/path, ID and icon through extraction, dissolution, unpacking and restart. Widgets and folders never become folder children.

`Orbit_Desktop_Launcher_UX_Spec_v2.md` is the current product specification and acceptance-test source. Its desktop Android-inspired spatial model supersedes the earlier list-only home-screen, Arch-only target and ordered-dock assumptions below. Existing Orbit appearance, item launching, search, native pickers and background-work requirements remain in force when compatible. New layout data lives in `config.json` version 5 with stable item IDs, signed Canvas rectangles, widget instances and a saved view; older schemas migrate on load while preserving rendered sizes. Grid item sizing is persisted in the existing Settings page. A dock pin owns a shortcut's layout location; the flat item collection retains its record for existing app/search/recent lookups.



Every decision for the MVP, in one place. Words follow `glossary.md`. Shortcuts are in `shortcuts.md`. Coding rules are in `rules.md`. What is not in the MVP is in `planned-features.md` and `roadmap.md`.

## 1. What Orbit is

Orbit is a fullscreen desktop app-launcher replacement for Windows and Arch Linux, styled like iOS. It replaces the desktop launcher and takes over the work that would otherwise send you to your browser bookmarks, your file search or your task app. You arrange apps, websites, files, directories, folders and widgets on an infinite Canvas, and open it instantly with a key. It starts empty. Search anything searches saved Canvas items, System actions, the cached Installed apps list, files in File search paths and immediate directories in Directory search paths by name. Ctrl+K focuses it. A Prefix routes the rest of the query to a site, a search engine, TickTick or a user Action, so the query is typed in Orbit and not in the browser's search field. Orbit is written in Rust with Slint and runs as one binary with a resident daemon that keeps everything ready, so showing the launcher does no disk or network work. Interactions and transitions are animated; showing and hiding the launcher is immediate.

## 2. Stack

- Rust and Slint. No web view. One binary.
- Goals, in order: extremely fast, minimal, simple (see `rules.md`).
- One of Orbit's goals is to minimize internet usage and Fetch requests. Save data locally where practical so Orbit remains normally usable offline; do not make routine use depend on internet access. Never Fetch automatically: only Fetch when the user explicitly requests it.
- Font: Inter, embedded in the binary. Glyph icons: Phosphor, loaded only while the Icon picker is open.
- Orbit is for personal use and is not distributed. If that changes, first check Slint's licence (GPLv3, or the royalty-free terms with attribution) and ship the licence texts for Inter (SIL OFL) and Phosphor (MIT).

## 3. Running Orbit

- `orbit daemon` starts the hidden resident process. It starts at login and keeps everything in memory, so showing the launcher does no disk or network work.
- `orbit toggle` shows or hides the launcher. If no daemon is running, it starts one and shows the launcher.
- Only one daemon runs. The commands reach it over a Unix socket.
- The launcher is a borderless, true fullscreen window on the monitor that has the cursor.
- Opening and closing the launcher keeps an already visible Hyprland special workspace open.
- It hides when: an app launches successfully, `Esc` is pressed with nothing else open, `orbit toggle` runs again, it loses focus, or the Close button in the top bar is clicked.
- You add `exec-once = orbit daemon` and `bind = SUPER, SPACE, exec, orbit toggle` to `hyprland.conf` (the key is yours). Orbit never edits that file. The app asks for fullscreen itself. An optional window rule can turn off Hyprland's open animation. Check the current rule syntax when you write the README.

**Speed test (step 1 is a go/no-go test).** Orbit must be as fast as possible, so step 1 measures before anything else is built.
- Time is measured from the moment the daemon receives the toggle to the first frame on screen. Budget: 50 ms.
- Memory is measured twice: hidden and idle (budget: under 140 MB), and visible. The radial background is embedded SVG artwork, cached by the renderer; remeasure after changing the visual assets.
- Try Slint's `show()` and `hide()` first. On Wayland they may destroy and recreate the window. If that misses the budget, find a faster way (for example keep the window and its contents alive) and write the result here.
- Compare Slint's software renderer with the GPU renderer for show time and memory. Keep the faster one and write the result here.

**Step 1 results** (Arch Linux, Hyprland 0.56.2, one 1920 × 1080 monitor at 48 Hz; 31 toggles per mode, release build; 2026-10-17). Time ends at Wayland presentation feedback for the first presented frame. Memory is resident set size (RSS), sampled from `/proc/<pid>/smaps_rollup`; MB is decimal. Each time is the first show, then the reopen median, 95th percentile and maximum.

| Renderer | Hide method | First show | Reopen median / p95 / max | Hidden and idle RSS | Visible RSS |
|---|---|---:|---:|---:|---:|
| Software | Destroy with Slint `hide()` | 21.6 ms | 18.9 / 28.0 / 33.2 ms | 16.9 MB | 29.2 MB |
| Software | Keep window on hidden Hyprland workspace | 22.9 ms | 15.9 / 24.8 / 28.0 ms | 33.6 MB | 33.6 MB |
| GPU (FemtoVG) | Destroy with Slint `hide()` | 19.6 ms | 42.9 / 52.9 / 53.7 ms | 110.5 MB | 110.5 MB |
| GPU (FemtoVG) | Keep window on hidden Hyprland workspace | 26.6 ms | 13.6 / 24.3 / 24.7 ms | 104.7 MB | 104.7 MB |

The GPU renderer with the window kept alive is used: it has the fastest reopen measurements (13.6 ms median) and stays within the 50 ms show budget. Its measured idle RSS is 122.8 MB in the current launcher, so the memory budget is 140 MB. Do not use Slint `hide()` because destroying the window raised the GPU reopen maximum to 53.7 ms.

After enabling FemtoVG and restarting the current release build on 2026-10-18, RSS was 122.8 MB both hidden and visible. The launcher was opened and hidden through `orbit toggle` and stayed resident on `special:orbit` while hidden.

## 4. Data model

```
Item   = App | Website | File | Directory | Folder
App    = { id, name, icon, launch }
Launch = Command(text) | Installed(installed app id) | Action(system action)
Icon   = Letter | Text(1–2 visible characters) | Svg(text) | File(name in icons/)
Website = { id, name, url, icon }
File   = { id, name, path, icon }
Directory = { id, name, path, icon }
Folder = { id, name, apps: [App] }
Canvas = { items: [Item], positions: { item id: rectangle }, view: { x, y, zoom } }
Dock   = { pinned: [item id] }      ordered
Recent = [item id]                  most recent first
```

- `config.json` has a `version` number, so later changes can read old files.
- No color property on apps or folders.
- A glyph chosen in the Icon picker is saved as `Svg` with dark gray written into the SVG, so nothing needs the glyph library afterwards. A pasted SVG keeps its own colors.
- `Icon::File` icons are only used for saved images, including system icons of installed apps and Website icons.
- An installed app stores a reference (its id in the Installed apps list), not a command. The Installed apps list holds the command to start it.
- Icons are decoded on the worker when an item is loaded or added and cached at no more than 80 × 80 px. The Installed apps list decodes cached system icons on the worker when first requested and keeps them ready. Opening the launcher never performs this work. The same installed app can be added more than once. Each copy is its own item with its own id.
- An Action is a named, reusable command with a description. Built-in Actions include URL templates and the TickTick task action; users can add shell scripts in Settings. A Prefix has a Name, Token and selected Action. Tab or Space activates a matching Token, hides search results and displays the Prefix name as a badge. Enter sends the remaining Search anything text to its Action.
- A Website opens its URL in the default browser. When it is added or saved again without an icon, Orbit ranks the page's declared SVG, large and touch icons, then tries the host favicon. Google's domain favicon is requested in parallel at 256 px so a blocked website cannot delay fallback. Connections are limited to two seconds, with six seconds for the whole attempt (including DNS) and 2 MiB per response. Redirected and relative icon URLs are supported; invalid images are skipped, and the best decoded result is used. The downloaded icon is saved in `icons/`. Fetching runs on the worker and never happens at startup or when showing the launcher.
- A File opens its path with the default app. A Directory opens its path in the default file manager. Both accept a typed absolute Path or a path selected with the native file chooser; the item name comes from the saved file or directory path. The default icon is a file or directory image, and can be changed in the Icon picker.
- A separate Action item is not needed. The predefined System actions are Apps; use Custom command for commands outside the Actions list. Search Prefixes dispatch to named Actions without parsing action-specific arguments in the search bar.

## 5. The launcher screen

The fixed controls sit above the full-window Canvas. The existing logo and wordmark remain at the left, Search anything is centered with its default engine selector beside it, and Lock layout, Settings and Close remain at the right. The floating Dock and Change background image button retain their positions. Only the controls' own bounds intercept Canvas input.

The Dock remains visible when empty. It shows up to five recent unpinned shortcuts and fifteen pinned items, a separator when both are present, 48 px icons, existing magnification, delayed tooltips and horizontal overflow scrolling. Showing the launcher clears search and closes folders, Settings and sheets while retaining the Canvas view.

## 6. Behavior

**Opening items.** Click, or `Enter` on the focused item.
- An app starts directly with no shell: the command is split into arguments and run detached, so the app survives a daemon restart. On success the launcher hides and the item enters Recent.
- A Website opens its URL in the default browser. A File opens with its default app. A Directory opens in the default file manager. These use `xdg-open` with the URL or path as one argument, without a shell. On success the launcher hides.
- On failure the launcher stays open and shows a banner such as "Couldn't start 'figma': not found".
- A folder opens instead of launching. Right-click empty Canvas space → Add item opens the Add item sheet.

**Context menu:** right-click a Canvas item for its existing Edit, Pin/Unpin, widget sizing/duplication and Delete operations. Delete asks for confirmation. Right-click empty space for Add item or Show all items. A pinned dock item offers Unpin; a Recent dock item offers Remove from Recent. There are no page-related actions.

**Sheets.** All open over a dimmed backdrop inside the launcher window. The Add item sheet fits the selected tab's content, has a maximum height of 760 px, and scrolls only when that content exceeds the available height.
- *Add item:* segmented control App, Website, File, Directory or Folder. For an App, a second segmented control: Custom command (first and selected by default), Actions or Installed apps. Actions shows Lock screen, Take screenshot, Sleep, Log out, Volume up, Volume down, Mute, Mute microphone, Play or pause, Next track and Previous track; choosing one adds it to the Canvas and closes the sheet. Audio and media actions use `wpctl` and `playerctl`, following the [Hyprland media examples](https://wiki.hypr.land/0.53.0/Configuring/Binds/) and [WirePlumber command reference](https://pipewire.pages.freedesktop.org/wireplumber/man/wpctl.html). Custom command has Name, Icon and Command, with Cancel and Save. Installed apps shows the saved Installed apps list and a Refresh button; clicking an app adds it right away near the clicked Canvas position and closes the sheet, with no Save. Adding the same installed app again is allowed. Its name and icon come from the list and can be changed later with Edit. Website has Name and URL. File and Directory each have an editable Path, a native chooser button, a default Icon with Change icon beside it, and Cancel and Save. The chooser fills Path. Save checks the path, derives the item name, adds it to the Canvas and closes the sheet. A Folder has just a Name, with Cancel and Save.
- *Edit app:* Name, Icon, Command (custom command apps only). Cancel and Save. Delete is in the item's Context menu.
- *Edit website:* Name and URL. Its cached icon is refreshed when the URL is changed.
- *Edit file* and *Edit directory:* editable Path, native chooser button, Icon and Change icon, with Cancel and Save. Choosing another path fills Path; Save updates the item and its name from that path. The Icon preview and Change icon button are vertically centered together in these sheets and Edit app. Delete is in the item's Context menu.
- *Edit folder:* Name. Cancel and Save. Delete is in the item's Context menu.
- Keys inside sheets, pickers and alerts are listed in `shortcuts.md`. Their defaults can be changed in Settings. `Enter` in a Name, URL, Path or Command field moves to the next field or Save. Clicking the backdrop closes the sheet.

**Alert.** Delete always asks for confirmation.
- Deleting an app removes it everywhere: the Canvas or folder, the dock.
- Deleting a Website, File or Directory removes that item from the Canvas and the dock. It does not delete the file or directory from the computer. Deleting a Website also removes its cached icon.
- Deleting a folder deletes only the folder. Its apps move to the Canvas where the folder was, at the folder's position. The alert says so, for example "Delete folder 'X'? Its 4 apps will move to the Canvas."
- If the deleted folder was pinned, it is unpinned. A folder with no apps is simply removed.

**Folders** work like iOS folders.
- They hold Apps, Websites, Files and Directories, and cannot nest. Widgets stay on the Canvas. The tile shows a small preview of its apps.
- A folder opens from its tile into a centered panel over the backdrop, with one scrolling grid. Clicking outside or pressing `Esc` closes it.
- Dropping an app on another app makes a folder with both. Dropping an app on a folder adds it. Websites, Files and Directories use the same folder hold gesture and retain their shortcut type.
- A folder disappears when its last app leaves it.
- Dragging an app out past the panel edge closes the folder visually and keeps the app under the pointer. Only releasing it over the Canvas moves it out of the folder; Esc cancels without changing the folder.

**Dock.**
- Pin gives the Dock exclusive ownership of an item. Unpin returns it to a nearby free Canvas position without displacing existing items.
- Right-click a pinned dock item to Unpin it. Right-click a Recent dock item to remove it from Recent history.
- Recent tracks successfully opened Apps, Websites, Files and Directories, including Apps inside folders. Folders themselves do not enter Recent. The five newest unpinned items appear on the left; pinned items appear on the right in saved order, up to fifteen. An item appears only once in the dock.
- Recent keeps up to sixteen item IDs so older unpinned items can fill the left side when a newer item is pinned or deleted. Opening an item again moves it to the front. A successful launch writes `recent.json` only when the order changes.
- Clicking a dock item opens it, including a folder or a shortcut inside a Folder. Dock drag reordering remains available.

**Drag and drop.** A click launches; a drag freely moves only the dragged item. Coordinates are transformed through the saved Canvas view. Occupied targets reject the drop unless the existing folder hold gesture is ready. Folder children preserve their type, URL/path, ID and icon through extraction, dissolution and restart. Folders and widgets cannot become folder children. A drag changes saved data only on release; Escape cancels without mutation. Pinned items belong exclusively to the Dock; dropping back onto the Canvas or Unpin gives them a free Canvas position.

Everything works with the mouse. Context menus stay inside the window. Sheets, alerts and folders close through their backdrop or Escape; Settings retains Back. Hover lightens tiles and the dock retains its magnification and tooltips.

## 7. Icons

The Icon picker has four parts, as requested on 2026-10-18:
- **Letter:** enter one or two visible characters, including emojis. Joined emojis count as one character. Existing automatic first-letter icons remain supported.
- **SVG:** paste SVG markup in a labeled text field.
- **URL:** shown disabled; fetching an icon URL is not implemented.
- **Choose icon:** opens the native file chooser for PNG, JPEG, SVG or ICO. Copy the selected image into Orbit's icons folder so it survives moving the source file.

Save validates and applies the icon draft; Cancel leaves the invoking sheet unchanged. Decoding runs on a worker. The Glyph picker remains deferred. The explicit request permits chosen icon images outside the earlier MVP restriction.

Tiles use the supplied lavender artwork. Letter icons have a lavender gradient and a pale letter. Saved SVG and system icons retain their artwork and fit the full tile. The supplied dock has separately drawn 48 px artwork for its matching built-in icons.

Installed apps use their real system icon, resolved once when you press Refresh and saved into `icons/`.

## 8. Installed apps list

- A saved index of programs found on the computer. **Only the Refresh button changes it.** No automatic refresh, no file watcher, no scan at startup, no scan when a sheet opens.
- Read `.desktop` files from the XDG data folders (`/usr/share/applications`, `~/.local/share/applications`, Flatpak exports). Skip `NoDisplay`, `Hidden` and `Terminal=true` (terminal apps are not in the MVP). Strip field codes such as `%u` and `%F`. Store each app's start command in the list.
- Resolve each icon from the icon theme once, at refresh. This is the one place where a small crate is justified, because icon themes are hard to resolve by hand.
- Refresh shows a status such as "4 new apps found". It never adds anything to the Canvas. You add apps from the list through the Add item sheet.
- Before the first refresh, the list is empty and shows a Refresh button.
- The list has a filter box that narrows it as you type. Clicking an app adds it to the Canvas immediately.

## 9. Settings

The top bar Settings control opens a dark panel with General, Shortcuts, Search bar, and Back. Add, Edit or Delete a path in either list. Each path is an existing folder and can be entered as an absolute Path. File search paths are searched recursively for files; Directory search paths show only their immediate directories. Indexing starts on the first non-empty search and stays cached until a path changes. The refresh status is visible in the panel. Esc returns to the launcher. Item size and Item Gap controls save to config.json; page margin is removed. Item tiles default to 128 px and allow 80–240 px; Item Gap allows 0–64 px. Startup controls remain deferred.

Settings also has Export data and Import data. Export saves a versioned JSON backup containing the Canvas, folders, saved icons, Pinned, Recent, search paths and the Installed apps list. Import opens the native chooser directly, then validates the chosen backup before replacing data. Cancelling the chooser leaves data unchanged. It retains the previous data in a sibling `orbit-before-import-<time>` directory and reloads the launcher. File and Directory items keep their paths; the referenced files and installed programs are not included. Both operations run on the worker and use a centered floating native file chooser. Orbit stays fullscreen behind the chooser, and returns focus to Settings when it closes. This explicitly requested addition is outside the original build order.

Settings has General, Shortcuts and Search bar tabs. Shortcuts lists each Orbit action once, with all assigned key combinations grouped in that row. Users can record, change, add and delete individual combinations. Duplicate key combinations are rejected. Bindings persist in config.json and backups; old configurations receive the defaults. Native text editing remains provided by text fields; the global toggle is configured in Hyprland. These user-requested changes extend the original fixed-shortcut scope.

## 10. Look

The authoritative visual reference is `doc/orbit-ui/orbit.png` and `orbit.svg` (1440 × 1080). The user approved this design replacing the previous light theme restrictions on 2026-10-18.

- Use the reference radial background, lavender artwork, logo, search field, Settings and Close controls and icon-only dock, with the approved Canvas tint and temporary review outlines.
- Embed Inter. Canvas labels are 14 px. Canvas icons are 92 px in the default 128 px tile, leaving a compact label gap; dock icons are 48 px.
- Top bar: logo at (48,40), 360 × 44 px centered search at y=34, controls 48 px apart at the right.
- Canvas items retain their default 128 px tile, 92 px icon and 14 px label. Position and resize are free, without a visible grid or enclosing surface border. Item Gap retains its role in default footprints and Folder spacing. Changing monitor dimensions does not reflow or resize saved Canvas positions.
- The dock is 64 px tall, 22 px above the bottom, with rounded corners and 68 px per item. Hovering grows the icon under the pointer to 1.6× and its neighbours by less with distance, and the icons slide apart and upward from the bar so they never overlap; the bar widens by the room they take. After hovering an item for 50 ms, a white rounded tooltip bubble with a downward caret shows the item's name above the icon. The dock can grow to 1400 px wide and scrolls horizontally when its Recent and Pinned items overflow. Side controls sit outside the scroll viewport. The separator divides Recent from Pinned when both are present.
- Search filters saved Canvas items, all System actions, the cached Installed apps list, files in File search paths and immediate directories in Directory search paths by name. Results from saved Canvas items appear after the other matches. The first result is initially selected. Arrow keys move the selection through the grid without wrapping and scroll it into view. Enter opens the selected result. Clicking any result opens it. Each highlight encloses the icon and labels with padding and a gap from neighboring results. Results show their saved or cached icon when available. Choosing an action runs it, and choosing an installed app launches it without adding it to the Canvas. Esc clears search before hiding.
- Prefix tokens activate with Tab or Space, display the Prefix name as a badge and hide search results while active. Enter sends only the remaining query as the Action argument. When no local results match, Enter searches with the web search engine selected in Settings → Search bar → General (Perplexity by default). Prefixes, Actions and the selected engine are stored in config.json; one Action can have several editable Prefix Tokens, and URL Actions encode query text. Site-search defaults include ChatGPT (`g`, `ل`), Claude (`c`, `ؤ`), GitHub (`gh`), Google Maps (`m`), Image Search (`i`, `ه`), Perplexity (`p`, `ح`), Wikipedia (`w`, `ص`), Translate ar to en (`ف`), Translate en to ar (`tr`), YouTube (`y`, `غ`), plus the web search engines. TickTick task creation parses `!today` (or `!!`), `!high`, `!medium`, `!low`, `!none`, `#tag`, and `^List`/`~List` in its own handler; omitted lists use Inbox. Shell Actions receive the query as a separate process argument, never interpolated into script source.
- Arrow keys navigate search results or spatial neighbors; they never change pages. Ctrl+K focuses search.
- Preserve native, interactive controls. The exported `main.slint` is only a full-screen PNG and cannot be used as the application implementation.
- Sheets, the Context menu and Delete alerts follow the supplied Add item reference in `doc/add-item` and the shared rules in `design-system.md`. The 2026-10-18 request adds system icons to Installed apps rows.

**Animation.** Short and few, because Orbit must stay fast.
- Showing and hiding the launcher: none. It is instant.
- Folder open: instant, with no movement or fade. Folder close: the panel shrinks back to its tile in about 220 ms.
- Drag: other items slide into the gap, about 120 ms, and the dropped item settles into place.
- Sheet and alert: the backdrop and panel fade in, about 120 ms.
- Focus ring: none. Hover: Canvas tiles lighten, dock icons magnify and slide apart over 80 ms with ease-out motion, and the hovered item's name appears in a tooltip after 50 ms.
- Input never waits. Keys and clicks take effect immediately and the animation only follows. There is no animation setting in the MVP.

## 11. Saved files

One folder: `~/.config/orbit/` (respecting `XDG_CONFIG_HOME`).

| File | Holds |
|---|---|
| `config.json` | Version, Canvas view and positions, items, folders, pinned order and file and directory search paths |
| `recent.json` | Up to sixteen recent item IDs, newest first |
| `installed-apps.json` | The Installed apps list |
| `icons/` | Saved icon images |

Files are JSON and are written atomically (temporary file, then rename). A missing file starts empty, with no banner (this is the first run). A damaged file is renamed to `<name>.bad` (replacing an older `.bad`), Orbit starts empty, and a banner says so. The next save never overwrites the damaged file.

## 12. Supplied UI reference

`doc/orbit-ui` is the source of truth for the launcher appearance. Extract vector artwork into `ui/assets`, retain dynamic text and layout in Slint, and keep user data functional. Do not use the full screenshot as a launcher surface. Sample items are not first-run defaults.

The 2026-10-18 UI task explicitly overrides the earlier exclusions of search, Ctrl+K, footer hints, gradient backgrounds, colored tiles and dock labels. It also permits the minimal search, pinning and Settings behavior needed to make the reference controls work, outside the original step order. All other unrequested future features remain deferred.

## 13. Build order

Each step ends with an app that runs on Arch.

1. **Skeleton and speed test:** fullscreen Slint window, daemon, `orbit toggle`, hide on focus loss, monitor with the cursor. Run the speed test from section 3 and write the results there. Stop and report if the budget cannot be met.
2. **Data:** `config.json`, atomic writes, the empty launcher, the Canvas and the empty-space Add item menu.
3. **Apps:** the Add item sheet for custom commands, launching, letter and SVG icons, the context menu, the delete alert.
4. **Folders and drag and drop:** folders and free Canvas placement. This is the largest step.
5. **Item types, App sources and Search Prefixes:** Websites, files and directories; Actions and custom commands; scan, cache and choose installed apps from the Add item sheet. Add reusable shell-script and URL-template Actions, editable Actions and Prefixes settings, generic token activation/dispatch, the TickTick task Action, and default site-search prefixes.
6. **Dock:** Recent, Pinned and pin reordering.
7. **Keyboard:** the few keys in `shortcuts.md` and the focus ring.
8. **Settings and looks:** appearance, background color and iOS polish.
9. **Glyph picker:** the Phosphor Icon picker.
10. **Packaging:** the `PKGBUILD` and the README.

### Robot extension (authorized 2026-11-03)

Implement one step at a time using `robot-capabilities.md`: (1) Robot data and its backup boundary, (2) local approvals, Skills, MCP and web tools, (3) multi-turn execution and queues, (4) Canvas Robot items, Settings and workbench, (5) full backup/runtime coordination and release acceptance. All five steps are needed for the first Robot release. Saved Teams and delegation follow in a later step.

## 14. Assumptions to confirm

1. Item size: Use 92 px icons in the default 128 px tile, 14 px labels and 144 px spacing between item centers. Settings can change square Canvas tiles from 80 to 240 px; icons scale to fit.
2. Editing an installed app changes its name and icon, not what it launches.
3. If an installed app is gone from the Installed apps list after a refresh, its item stays on the Canvas. Launching it shows a banner such as "Couldn't start 'X': not found".

## 15. Open questions

None at the moment.

Settings groups Home Screen sizing, TickTick account and data under General. Search bar contains General (search engine and search paths), Prefixes and Actions. TickTick widgets use the raised surface background.

Search anything wraps long queries and grows vertically, with results below the field. Backspace removes an active Prefix when its query is empty. When local items match, show those results. If none match, show no search engine tiles; Enter still searches with the saved default engine (Perplexity initially). The default engine select beside Search anything opens the engine list.

Search engine results use locally bundled brand icons in rounded tiles.

## Smooth cursor

Settings → Search bar → General → Smooth cursor saves immediately and defaults to On, including existing config files. Search anything uses a gliding block and bounded trail without delaying text or search. It fades softly while idle, hides for selection and snaps on line changes and reopening. Hidden or unfocused cursors perform no idle work. Native editing, selection, clipboard, shortcuts and IME are preserved; IME composition and Off use the native caret.

Settings → Shortcuts includes Delete previous word (Ctrl+W by default), scoped to focused Search anything. Native word boundaries, selection and undo are preserved. Existing configs receive the default once when Ctrl+W is free; removing it keeps it removed.

Shortcut displays use shared native Kbd keycaps, including search, footer, Settings bindings and recording. Search and close hints reflect saved bindings. Search anything reserves full cursor space at wrapped line ends and scrolls the entire final block into view, with or without a Prefix.

Search anything uses 16 px text for the query and placeholder. Opening a folder animates its panel from the tile to the centered overlay over 220 ms while the backdrop fades in; closing reverses the movement.


### Settings redesign and file-search exclusions (2026-10-22)

Settings follows `settings/DESIGN.md` with General, Shortcuts, Search bar and Back in a sidebar, and General, Prefixes and Actions in Search bar’s segmented control. The top bar and dock retain their existing appearance with the approved temporary review outlines. Existing actions, sizing preview, Save, Reset and unsaved-sizing cancellation remain available. Shortcuts use display-only key chips with × and confirmation; use Add after removing a binding to change it.

Settings → Appearance lets the user save a solid background color as `#RRGGBB`, choose a PNG, JPEG or SVG background image, and Reset background to restore the supplied radial SVG. The selected image is copied into Orbit's saved icon storage and decoded by the existing worker; showing the launcher does no file I/O or image decoding.

Appearance also saves a Background images folder, defaulting to `/home/sebaq/Pictures/aesthetic-wallpapers/images`. The image-change icon at the bottom right of the Home Screen selects a different PNG, JPEG or SVG from that folder; image reads and decoding stay on the worker.

Search bar → General includes Excluded from file search. `config.json` version 3 adds `file_search_exclusions`, defaulting when missing to `.git`, `node_modules`, `.*`, `.local`, `.claude`, `.agent`, `.cache`, `.venv`, `__pycache__`, `dist`, `build`, `.next`, `target`, `.idea`, `.vscode`, `.npm`. An explicitly empty list stays empty. Trim entries and ignore blanks and duplicates; successful edits save atomically and invalidate the worker’s cached path index. Errors preserve the draft and saved list.

Patterns match whole descendant file or directory names case-sensitively. Only `*` is special and matches zero or more characters; `.*` matches dot-prefixed names. Matching directories are not descended into. Explicitly configured search roots are still entered. Directory search and saved launcher items are unaffected. Index rebuilding stays on the worker and does no work when showing Orbit.


## Layout lock (2026-10-22)

Lock layout / Unlock layout is the padlock immediately left of Settings. One saved state applies to the Canvas, folder contents and the pinned dock; existing configurations default to unlocked. While locked, disable dragging, resizing, pin/unpin, adding, duplicating, deleting and rearranging items, layout sizing controls and Import data. Rust handlers reject these operations as well as the interface. Navigation, launching, search, scrolling, Recent updates, widget interactions and editing existing content remain available. Locking cancels pending drag/resize and placement previews. The lock survives hiding and restarting Orbit; no default shortcut is added.

### Website icon drafts and anchored Add item (2026-10-22)

Add item has a stationary top edge based on its maximum height (760 px, viewport permitting); type changes animate the bottom edge over 180 ms. Website and Edit website show an icon preview before saving. Automatic fetch is the default, debounced while editing URL and performed on a worker; users can choose/change a custom icon or return to Automatic. Save reuses the completed preview, and fetch failures permit saving a letter fallback. Closing or changing sheets invalidates obsolete previews. Custom image names retain the existing chosen-* convention, so mode is inferred without a config migration.

## Local English assistance (2026-10-22)

The approved staged behavior is in `english-assistance.md`: a selected local Ollama generation model (default `qwen3.5:2b`) supplies sentence suggestions, then Ctrl+E next-word completion, then English grammar explanations with clickable corrections. Features are independent and default disabled in Search bar → English assistance. Sentence suggestions appear above ordinary search results after two words, and replace text only when explicitly chosen. Model work is asynchronous, text snapshots reject stale results, syntax is protected, and native undo remains available. Cloud support and automatic replacement are deferred.

English assistance now supports choosing installed local Ollama generation models and editing the three system prompts in Settings → Search bar → English assistance. Ctrl+E and Tab accept a visible next word; exact Prefix Tokens retain Tab priority. The select outside the left of Search anything displays and saves the default search engine.

## AI Prefix

The built-in AI Action has the default Token `ai`. Space or Tab activates it; Enter or Ask sends a nonempty prompt and keeps Orbit open. The answer panel offers a provider/model picker, a scrollable selectable plain-text answer, Copy and Cancel. Every submission starts fresh. Cancelling, editing a pending prompt, changing provider/model, entering Settings, leaving the Prefix or hiding Orbit invalidates pending replies. Leaving AI mode clears the in-memory answer. No conversation history is saved.

Settings → Search bar → AI configures Ollama, OpenAI and Gemini independently of local-only English assistance. Ollama initially inherits its address/model and accepts only loopback HTTP endpoints and installed completion-capable local models. Refresh models is explicit. Cloud model IDs are entered explicitly; there is no paid default or automatic provider fallback. Keys are entered through masked fields and saved/replaced/removed in the OS credential store, never config.json. Cloud API usage is separately billed.

Requests run on a dedicated worker with a 120-second request timeout. Cancel invalidates results immediately; an already running HTTP request may finish or time out before the next queued request starts. Only the submitted prompt is sent, with no files, inventory, conversation history or tools. OpenAI requests use store=false. Replies are fetched in full, not streamed, and never executed or logged. Normal launcher opening does no model loading or AI network work.

Existing configurations receive the AI Action and Prefix once without replacing a user-owned ai Token or existing Action ID. Later removal is preserved. Preferences persist separately from English assistance. Authentication, limits, unavailable models, blocked/empty replies and connection failures are recoverable in the panel.

AI verification (2026-10-24): native input in an isolated Hyprland release session exercised Settings → Search bar → AI, local model refresh/selection, ai activation, submission, the visible local-model answer, Copy and Cancel. Clipboard output matched the displayed answer. OpenAI and Gemini model IDs saved independently; the masked key draft stayed out of config.json and cleared on provider changes. No cloud keys were created or saved, and live cloud calls remain unverified until keys are configured. Automated checks cover request bodies, responses, HTTP failures, timeouts, migrations and stale replies. Captures and the check report are in target/ui-check/ai-*.

AI mode uses a dedicated opaque answer card aligned with a wider prompt field. While its Prefix is active, Canvas items/widgets, search results/details and the search-engine chooser are hidden, including for an empty prompt. Hidden-result activation is disabled; the dock stays available. Leaving AI mode restores the normal launcher. Empty/loading/error content stays compact; answers grow to a bounded height and scroll with native selection and Copy.

AI view verification (2026-10-24): the rebuilt release was opened on a populated page with TickTick widgets. Both Space and Tab activated the clean AI view; a visible local-model answer and exact clipboard copy were checked, a 50-line reply scrolled through its final line, arrow-key editing worked, and Esc restored the populated page. Formatting, Clippy and all 91 enabled tests passed (6 existing opt-in tests ignored). Captures and details: target/ui-check/ai-clean-*.

UI update (2026-10-25): Add item → Widgets is a responsive card grid. Footer shortcut hints are removed. The dock fits its items with equal left and right padding; an empty dock remains visible. Shared Theme tokens govern repeated type, corner and feedback values across the interface.

Settings → Search bar → General groups App search paths, File search paths, Directory search paths and Excluded from file search in one Search locations panel. Refresh rebuilds Installed apps and rescans all configured file and directory paths on the worker, applying file exclusions. All installed-app refresh controls use this shared operation. Newly created files become searchable after Refresh without restarting Orbit.

Search results for saved shortcuts, System actions, Installed apps, Files and Directories offer Pin and Add to Canvas in their Context menu. Add to Canvas uses the visible center or a nearby free position. Dragging a result hides search results and English assistance while revealing the Canvas; release on free Canvas space saves its exact placement, and release on the dock pins it. Existing saved items move by ID instead of creating a duplicate; new results become saved shortcuts. Invalid drops and Esc cancel without saving. Search engine fallback results cannot be added. Lock layout prevents these changes. Successful placement clears Search anything.

Morning Remembrances (Adhkar), explicitly requested 2026-10-27, adds a local widget with the user's 25 Arabic entries. Each tap counts one repetition; reaching the required count plays a chime and advances automatically. Only completing every entry plays the distinct final chime and shows Restart. Skip retains unfinished entries, Previous preserves counts, and source details expand. Daily per-instance progress and editable content live in widgets.db (schema 3); the bundled source JSON remains unchanged. Add it through Add item → Widgets → Morning Remembrances → Auto or Place.

Evening Remembrances (Adhkar), requested 2026-10-27, adds the supplied 25 evening entries as a separate widget and Settings → Data collection. It reuses Morning Remembrances' fixed 80 px counter, text scrolling, source details, navigation, sounds and daily per-instance progress. The gallery offers Auto and Place with the same sizing. Database schema 4 seeds evening entries once without replacing morning content or progress. Both collections remain offline and independently editable. See `widgets.md` for acceptance checks.

Completing all repetitions in a Morning Remembrances or Evening Remembrances widget adds that collection to the daily Achievements, once per widget. Restarting preserves the earned completion.

Azan Reminder (2026-10-28) has a settings icon for per-widget City, Country and Time zone, defaulting to Sohag, Egypt and Africa/Cairo. Fetch times retrieves today's Aladhan method-5 schedule for review; Save accepts the preview locally. No automatic Fetch occurs. Named zones govern dates and countdowns, including seasonal clock changes. Missing or stale schedules withhold countdowns. Existing schedules remain in Settings → Data. The widget emphasizes the next prayer and time above aligned daily rows.

TickTick widget update (2026-10-28): the header picker scrolls through Today, Next 7 Days, all tasks, Lists and Tags. A tag selection filters tasks by the exact tag and persists per widget. Header add/menu controls use matching inset icon buttons. High, Medium and Low priorities use red, amber and blue checkboxes; no priority uses the neutral checkbox. Arabic-first titles use RTL row placement, right alignment and the embedded Arabic font. Wheel and trackpad scrolling uses a higher proportional scroll distance with no interpolation delay, so rapid wheel input accumulates immediately.

Focus task redesign (2026-10-28): the TickTick Focus Task card uses a wrapping task title, quiet list context, priority/date metadata, tags and readable notes/checklists. A circular priority-colored completion control stays at the upper right, and Open task opens the existing editor. Details reveals the full task metadata inside the same scrollable body. Short cards use a compact title layout. Arabic titles and notes keep their own text direction. Header dragging, resizing and immediate wheel scrolling remain available.

English assistance displays only the untyped completion suffix; corrections to typed letters use an explicit Autocomplete replacement preview. Ctrl+E, Tab and Accept word accept the same validated current-word or next-word edit. Grammar corrections remain separate clicks. Known built-in prompts migrate without replacing custom prompts; grammar defers the word still being typed. See `english-assistance.md` for the 2026-10-30 repair and verification.

English Suggestions show Ctrl+1/2/3 keycaps for direct application. Ctrl+N/J moves the highlight down, Ctrl+P/K moves it up, and Enter applies the selected rewrite without submitting. The selection resets on refreshed or invalidated results. Ctrl+K keeps its normal focus action outside current suggestions.

## Robots (authorized 2026-11-03)

The first release adds persistent Robots while retaining the independent AI Prefix. Add item offers Robot with one shared 2D illustration. Opening a Robot shows its workbench over the Canvas: selected project, conversations, progress, tool activity and results. Settings manages Robot profiles, reusable Roles, model selection, installed Codex Skills and local stdio MCP servers. Multiple profiles can be saved, but only one Robot executes at a time. Teams are deferred.

Choose an existing Ollama, OpenAI or Gemini model. New conversations copy their starting model, provider, Role instructions and project path; resuming retains that context even after profile edits. Commands queue per Robot, continue while Orbit is hidden and persist replies and tool activity locally. Conversation titles derive from the first message, history orders by recent activity, deletion is permanent in the live database and optional one-to-five-star ratings are saved as feedback.

Orbit implements the runtime and executes selected Skill workflows using its own tools. Every local file or shell operation requires approval for that exact action. The project is a working directory, not a sandbox: approved tools may access the machine. Enabled MCP tools execute automatically. Historical approvals never authorize replay after restart or import. Internet research uses built-in web search and MCP tools, retains source links and writes a requested Markdown report only after local-write approval. Credentials stay in the OS keyring.

`robots.db` schema 2 stores profiles, Roles, immutable conversation context, activity and ratings transactionally, separately from Canvas configuration and widget data. History reads 50 summaries at a time; conversations are read on demand by workers. No Robot history or Skill scanning runs when showing the launcher. Backup version 3 includes a validated Robot snapshot, queued-command states and credential references, never keyring values. Versions 1 and 2 remain readable. Import retains previous data, stages all restored records and clears MCP enabled-tool lists until the user re-enables them. Version 1 backups remain readable. Existing backup limits and the widget-database exclusion remain unchanged.

The later Teams step adds saved lead/specialist Robots, direct assignment and lead delegation. Each assignment creates one team run, with sequential member work in the shared project folder and one combined history.

Release acceptance follows the user's checks: create/reopen a configured Robot and Role; queue work, approve a local action, observe automatic enabled MCP activity and completion while hidden; verify a sourced Markdown research report; round-trip profiles, conversations and ratings without credential values; verify Canvas use and Arabic/English/mixed text through the real entry path on an isolated display/profile; pass formatting, Clippy and a release build, then restart the normal daemon hidden. Teams have separate later acceptance for direct assignment, delegation and sequential shared-folder edits.

Implementation status: steps 1–3 provide storage, backup format, worker-side tools and the multi-turn runtime. Local UTF-8 reads, directory listings, atomic text writes, program/shell execution and installed Skill loading require a single-use approval of the exact action and project directory. MCP stdio discovery and enabled-tool calls use protocol 2025-06-18, bounded messages, deadlines and keyring-injected environment values. Built-in Brave Search returns titles, snippets and source URLs independently of the model provider; its API key uses `orbit-robots` / `brave-search` in the OS keyring. Reports are saved through the approved local write tool. Step 4 connects Add item → Robot and Settings to profile and Role management, Skill paths, MCP discovery/configuration, credential storage and the conversation workbench. The workbench exposes queued commands, exact-action approvals, activity, results, history, ratings and permanent deletion. The isolated Linux workbench flow is verified with local model/MCP fixtures, including Arabic text, queued work, approvals, saved reports, restart persistence, ratings and deletion. Full backup/runtime release acceptance remains step 5; live provider/search and Windows checks remain outstanding. See `adr/0002-robot-execution.md` for the execution and permission model.


Robot runtime (step 3): one worker executes accepted commands in FIFO order across Robot profiles, with up to 128 waiting commands. Acceptance is reported only after a Queued entry is saved. Later queued commands are excluded from the active model context. All local approvals match both conversation and exact in-memory action ID, are single-use, and occupy the one execution slot while waiting. A denied Skill stops its workflow; a denied ordinary action returns denial to the model and cannot be requested identically again in that command. Execution stops if history cannot be saved. Cancelling discards pending approvals and queued commands for that conversation; already executing calls may finish or time out, but no subsequent action starts. Closing an observer does not stop the worker.

Ollama uses native tool calls and verifies the selected installed model is local and tool-capable. OpenAI uses Responses with store=false and opaque encrypted reasoning continuation; Gemini retains native function-call parts and thought signatures. Opaque provider state remains in memory for the current command. Resumed conversations rebuild context from saved commands, replies and historical tool results while retaining the starting model and Role. Private reasoning is not saved as conversation text. There is no provider fallback or automatic retry of failed actions. A command stops after 32 model turns; each turn allows up to 16 tool calls and each request at most 8 MiB. Tool results sent to the model/history are capped at 512 KiB with an explicit truncation notice; selected Skill instructions retain the existing 1 MiB limit.

On first runtime use, unfinished queued/running commands are marked Interrupted without replay. The schema-1 to schema-2 upgrade preserves existing tables and records; version 3 backups record the new command states. Import remains nonexecuting. Import rejects outstanding Robot work, synchronously stops the idle runtime and closes its database before replacing data, and prevents new work during replacement. Reopening imported data marks unfinished commands Interrupted without replay. Runtime tests use isolated databases rather than the normal profile.

## Robot provider accounts (authorized 2026-11-03)

The provider extension in `../robot-provider-capabilities.md` is approved. Implement its steps in order: (1) named accounts and conversation/backup binding, (2) connections and sign-in/model discovery, (3) native agent execution and approval/cancellation routing. This extension authorizes provider subprocesses in the later execution step, superseding the older no-Codex-spawning restriction; Orbit must not depend on T3 Code itself.

Step 1 adds saved named accounts for Ollama, OpenCode, Antigravity, GitHub Copilot and Codex/ChatGPT while retaining OpenAI API and Gemini. Settings → Robots → Profile selects an account, edits its metadata and assigns it when the Robot profile is saved. Multiple accounts may use one provider. Ollama accounts store a loopback address; OpenAI/Gemini accounts store an OS credential reference; native accounts store a separate absolute account directory reference. Actual authentication files and credential values are never exported. The existing Credentials tab saves referenced API keys in the system store. Profiles without an account retain their legacy AI settings.

Each new conversation resolves the selected saved account and copies its connection into its immutable model context. Later Robot/account edits do not retarget existing conversations. Missing accounts fail explicitly; referenced accounts cannot be removed by the storage API. Robot database schema 3 adds an accounts table without rewriting existing Robot or conversation records. Backup version 4 includes account metadata and reads versions 1–3; import marks account records and captured contexts as requiring reconnection. Explicitly saving the account configuration authorizes its use again; a saved conversation retains its original connection/credential references. Native sign-in and execution remain unavailable until steps 2–3 and return an explicit error, without spawning a program or switching providers.

Provider account step 1 verification (2026-11-04): 172 tests passed, with 5 existing opt-in tests ignored; formatting, Clippy and the release build passed. Isolated UI checks verified two independently responding Ollama account fixtures, Robot account switching, original-account conversation resumption, Arabic/mixed text and restart persistence. A fresh Add item → Robot path verified native account metadata, the visible account-name summary and explicit unavailable execution without tool calls. The normal daemon was restarted hidden. Evidence is recorded in `../robot-provider-capabilities.md`. Native sign-in and execution remain steps 2–3; live cloud accounts and Windows remain unverified.

Tab and Enter script-name completion append a space when no argument separator exists, placing the cursor ready for arguments. Existing argument text and spacing are preserved.
