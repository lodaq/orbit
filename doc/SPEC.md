# Orbit spec

Every decision for the MVP, in one place. Words follow `GLOSSARY.md`. Shortcuts are in `SHORTCUTS.md`. Coding rules are in `RULES.md`. What is not in the MVP is in `PLANNED_FEATURES.md` and `ROADMAP.md`.

## 1. What Orbit is

A fullscreen app launcher for Arch Linux (Hyprland), styled like iOS. You arrange apps and folders on pages, like a phone home screen, and open it instantly with a key. It starts empty. Search anything searches saved page items, System actions, the cached Installed apps list, files in File search paths and immediate directories in Directory search paths by name. Ctrl+K focuses it. Windows is not in the MVP (see `PLANNED_FEATURES.md`).

## 2. Stack

- Rust and Slint. No web view. One binary.
- Goals, in order: extremely fast, minimal, simple (see `RULES.md`).
- Font: Inter, embedded in the binary. Glyph icons: Phosphor, loaded only while the Icon picker is open.
- Orbit is for personal use and is not distributed. If that changes, first check Slint's licence (GPLv3, or the royalty-free terms with attribution) and ship the licence texts for Inter (SIL OFL) and Phosphor (MIT).

## 3. Running Orbit

- `orbit daemon` starts the hidden resident process. It starts at login and keeps everything in memory, so showing the launcher does no disk or network work.
- `orbit toggle` shows or hides the launcher. If no daemon is running, it starts one and shows the launcher.
- Only one daemon runs. The commands reach it over a Unix socket.
- The launcher is a borderless, true fullscreen window on the monitor that has the cursor.
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
Page   = { items: [Item] }
Dock   = { pinned: [item id] }      ordered
Recent = [item id]                  most recent first
```

- `config.json` has a `version` number, so later changes can read old files.
- No color property on apps or folders.
- A glyph chosen in the Icon picker is saved as `Svg` with dark gray written into the SVG, so nothing needs the glyph library afterwards. A pasted SVG keeps its own colors.
- `Icon::File` icons are only used for saved images, including system icons of installed apps and Website icons.
- An installed app stores a reference (its id in the Installed apps list), not a command. The Installed apps list holds the command to start it.
- Icons are decoded on the worker when an item is loaded or added and cached at no more than 80 × 80 px. The Installed apps list decodes cached system icons on the worker when first requested and keeps them ready. Opening the launcher never performs this work. The same installed app can be added more than once. Each copy is its own item with its own id.
- An Action is a predefined app launch, selected from the Actions list. Custom command is the choice for commands outside that list.
- A Website opens its URL in the default browser. When it is added or saved again without an icon, Orbit reads the page's declared icon, then tries the host favicon and Google's domain favicon. The downloaded icon is saved in `icons/`. Fetching runs on the worker and never happens at startup or when showing the launcher.
- A File opens its path with the default app. A Directory opens its path in the default file manager. Both accept a typed absolute Path or a path selected with the native file chooser; the item name comes from the saved file or directory path. The default icon is a file or directory image, and can be changed in the Icon picker.
- A separate Action item is not needed. The predefined System actions are Apps; use Custom command for commands outside the Actions list.

## 5. The launcher screen

From top to bottom: top bar, the open page, page dots, dock.

- **Top bar:** the supplied Orbit logo and wordmark (`ui/assets/orbit.svg`) and name on the left. The Settings and Close icons on the right.
- **Page:** a full-width, full-height page area below the top bar and above the dock, with a grid of items and the Add tile last. It scrolls vertically and has no item limit.
- **Page dots:** show which page is open. Clicking a dot opens that page. A + beside the dots adds a page, as does Add page in Settings.
- **Dock:** a floating, semi-transparent white rounded bar at the bottom center, with no blur. Up to two recent items appear on the left and pinned items in saved order on the right, separated by a thin line when both sections have items. Each 48 px icon has an 11 px name below it. The dock is always shown, even when it is empty.
- **On open:** the launcher opens on its current page with folders, Settings, sheets and alerts closed, search cleared, and Search anything focused. Grid focus navigation remains deferred.

## 6. Behavior

**Opening items.** Click, or `Enter` on the focused item.
- An app starts directly with no shell: the command is split into arguments and run detached, so the app survives a daemon restart. On success the launcher hides and the item enters Recent.
- A Website opens its URL in the default browser. A File opens with its default app. A Directory opens in the default file manager. These use `xdg-open` with the URL or path as one argument, without a shell. On success the launcher hides.
- On failure the launcher stays open and shows a banner such as "Couldn't start 'figma': not found".
- A folder opens instead of launching. The Add tile opens the Add item sheet.

**Context menu** (right-click an item): Edit, Pin (or Unpin), Delete on page items and folder apps. A pinned dock item offers Unpin; a Recent dock item offers Remove from Recent.

**Sheets.** All open over a dimmed backdrop inside the launcher window.
- *Add item:* segmented control App, Website, File, Directory or Folder. For an App, a second segmented control: Custom command (first and selected by default), Actions or Installed apps. Actions shows Lock screen, Take screenshot, Sleep, Log out, Volume up, Volume down, Mute, Mute microphone, Play or pause, Next track and Previous track; choosing one adds it to the current page and closes the sheet. Audio and media actions use `wpctl` and `playerctl`, following the [Hyprland media examples](https://wiki.hypr.land/0.53.0/Configuring/Binds/) and [WirePlumber command reference](https://pipewire.pages.freedesktop.org/wireplumber/man/wpctl.html). Custom command has Name, Icon and Command, with Cancel and Save. Installed apps shows the saved Installed apps list and a Refresh installed apps button; clicking an app adds it right away at the end of the current page (before the Add tile) and closes the sheet, with no Save. Adding the same installed app again is allowed. Its name and icon come from the list and can be changed later with Edit. Website has Name and URL. File and Directory each have an editable Path, a native chooser button, a default Icon with Change icon beside it, and Cancel and Save. The chooser fills Path. Save checks the path, derives the item name, adds it to the current page and closes the sheet. A Folder has just a Name, with Cancel and Save.
- *Edit app:* Name, Icon, Command (custom command apps only). Delete and Save.
- *Edit website:* Name and URL. Its cached icon is refreshed when the URL is changed.
- *Edit file* and *Edit directory:* editable Path, native chooser button, Icon and Change icon, with Delete and Save. Choosing another path fills Path; Save updates the item and its name from that path. The Icon preview and Change icon button are vertically centered together in these sheets and Edit app.
- *Edit folder:* Name. Delete and Save.
- Keys inside sheets, pickers and alerts are listed in `SHORTCUTS.md`. They are fixed. `Enter` in a Name, URL, Path or Command field moves to the next field or Save. Clicking the backdrop closes the sheet.

**Alert.** Delete always asks for confirmation.
- Deleting an app removes it everywhere: its page or folder, the dock.
- Deleting a Website, File or Directory removes that item from its page and the dock. It does not delete the file or directory from the computer. Deleting a Website also removes its cached icon.
- Deleting a folder deletes only the folder. Its apps move to the page where the folder was, at the folder's position. The alert says so, for example "Delete folder 'X'? Its 4 apps will move to the page."
- If the deleted folder was pinned, it is unpinned. A folder with no apps is simply removed.

**Pages.**
- No names. At least 1, at most 9.
- A new page is created by dropping an item past the right edge of the last page, clicking + beside the page dots, or clicking Add page in Settings.
- Drag an empty area of the page left or right to move to the previous or next page. The page follows the pointer without delay and settles into place after release. At the first or last page, it moves only a little and returns to the center.
- A 24 px gap separates page surfaces during a swipe.
- The page area fills the available width and height. The + beside the page dots creates and opens an empty page with the mouse.
- Leaving an empty page does not remove it.
- First run: one empty page with the Add tile.
- Startup page controls are deferred; reopening keeps the current page.

**Folders** work like iOS folders.
- They hold apps only and cannot nest. Websites, files and directories stay on pages and cannot be placed in folders. The tile shows a small preview of its apps.
- A folder opens from its tile into a centered panel over the backdrop, with one scrolling grid. Clicking outside or pressing `Esc` closes it.
- Dropping an app on another app makes a folder with both. Dropping an app on a folder adds it. Websites, files and directories can be reordered and moved between pages, but do not make folders.
- A folder disappears when its last app leaves it.
- Dragging an app out past the panel edge closes the folder and places the app on the current page.

**Dock.**
- Pin or Unpin in an item's context menu adds or removes it from the dock. The item stays on its page.
- Right-click a pinned dock item to Unpin it. Right-click a Recent dock item to remove it from Recent history.
- Recent tracks successfully opened Apps, Websites, Files and Directories, including Apps inside folders. Folders themselves do not enter Recent. The two newest unpinned items appear on the left; pinned items appear on the right in saved order, up to eight. An item appears only once in the dock.
- Recent keeps up to sixteen item IDs so older unpinned items can fill the left side when a newer item is pinned or deleted. Opening an item again moves it to the front. A successful launch writes `recent.json` only when the order changes.
- Clicking a dock item opens it, including a folder or an App inside a folder on another page. Dock drag reordering remains a later functional step.

**Drag and drop** (iOS behavior).
- A click without movement launches. Dragging makes the other items slide aside to open a gap, and dropping fills it.
- Holding an app over the middle of another app for about half a second makes a folder. A folder dropped onto anything only reorders.
- Holding a dragged item at the left or right screen edge for about half a second shows the neighboring page. At the right edge of the last page, it shows a new empty page (if there are fewer than 9). Dropping inserts the item at the pointer's position and opens that page. Near the top or bottom edge, the page scrolls.
- Dropping on the dock snaps the item back. `Esc` during a drag cancels it.

**Everything works with the mouse.** Orbit is mouse first, and the keyboard only repeats what the mouse can do.

| To do this | With the mouse |
|---|---|
| Open an item (launch an app, open a Website, File or Directory; open a Folder) | Click it |
| Add an item | Click the Add tile |
| Edit, Pin or Unpin, Delete | Right-click the item, choose from the context menu |
| Reorder, move between pages, make a folder | Drag and drop |
| Change page | Click a page dot, or Shift + wheel |
| Add a page | Click + beside the page dots, or Add page in Settings |
| Open Settings | Click the Settings icon |
| Leave Settings | Click Back |
| Hide the launcher | Click the Close icon |
| Close a sheet, alert, picker or folder | Click Cancel or the backdrop |

Hover shows a slightly lighter tile and the pointer cursor, so it is clear what can be clicked.

## 7. Icons

The Icon picker has four parts, as requested on 2026-10-18:
- **Letter:** enter one or two visible characters, including emojis. Joined emojis count as one character. Existing automatic first-letter icons remain supported.
- **SVG:** paste SVG markup in a labeled text field.
- **URL:** shown disabled; fetching an icon URL is not implemented.
- **Choose icon:** opens the native file chooser for PNG, JPEG, SVG or ICO. Copy the selected image into Orbit's icons folder so it survives moving the source file.

Save validates and applies the icon draft; Cancel leaves the invoking sheet unchanged. Decoding runs on a worker. The Glyph picker remains deferred. The explicit request permits chosen icon images outside the earlier MVP restriction.

Tiles use the supplied lavender artwork. Letter icons have a lavender gradient and a pale letter. Saved SVG and system icons retain their artwork and fit the full tile. The supplied dock has separately drawn 48 px artwork for its matching built-in icons.

Installed apps use their real system icon, resolved once when you press Refresh installed apps and saved into `icons/`.

## 8. Installed apps list

- A saved index of programs found on the computer. **Only the Refresh installed apps button changes it.** No automatic refresh, no file watcher, no scan at startup, no scan when a sheet opens.
- Read `.desktop` files from the XDG data folders (`/usr/share/applications`, `~/.local/share/applications`, Flatpak exports). Skip `NoDisplay`, `Hidden` and `Terminal=true` (terminal apps are not in the MVP). Strip field codes such as `%u` and `%F`. Store each app's start command in the list.
- Resolve each icon from the icon theme once, at refresh. This is the one place where a small crate is justified, because icon themes are hard to resolve by hand.
- Refresh shows a status such as "4 new apps found". It never adds anything to a page. You add apps from the list through the Add item sheet.
- Before the first refresh, the list is empty and shows a Refresh installed apps button.
- The list has a filter box that narrows it as you type. Clicking an app adds it to the current page immediately.

## 9. Settings

The top bar Settings control opens a dark panel with Add page, Refresh installed apps, file and directory search paths, and Back. Add, Edit or Delete a path in either list. Each path is an existing folder and can be entered as an absolute Path. File search paths are searched recursively for files; Directory search paths show only their immediate directories. Indexing starts on the first non-empty search and stays cached until a path changes. The refresh status is visible in the panel. Esc returns to the launcher. Appearance and startup controls remain deferred.

## 10. Look

The authoritative visual reference is `doc/orbit-ui/orbit.png` and `orbit.svg` (1440 × 1080). The user approved this design replacing the previous light theme restrictions on 2026-10-18.

- Use the reference radial background, lavender artwork, logo, search field, Settings and Close controls, circular dotted Add tile, page dots, labeled dock and footer hints.
- Embed Inter. Page labels are 14 px, dock labels 11 px. Page icons are 80 px; dock icons are 48 px.
- Top bar: logo at (48,40), 360 × 44 px centered search at y=34, controls 48 px apart at the right.
- Grid: 144 px between item centers at reference width. The page area fills the available width and height, and the grid adds columns as width permits. The page and its 128 × 128 px item tiles have 1 px borders. Item tiles have a lighter background on hover. The gaps between tiles do not select items.
- Page dots are centered 146 px above the bottom. The dock is 94 px tall, 26 px above the bottom, with 26 px corners and 100 px per item. Each dock item's hover area is inset 6 px on all sides. The separator divides Recent from Pinned when both are present.
- Search filters saved page items, all System actions, the cached Installed apps list, files in File search paths and immediate directories in Directory search paths by name. The first result is highlighted and selected; Enter opens it, and clicking any result opens that result. Results show their saved or cached icon when available. Choosing an action runs it, and choosing an installed app launches it without adding it to a page. Esc clears search before hiding.
- Left and Right switch pages when a text field, sheet or folder is not active. Ctrl+K focuses search.
- Preserve native, interactive controls. The exported `main.slint` is only a full-screen PNG and cannot be used as the application implementation.
- Sheets, the Context menu and Delete alerts follow the supplied Add item reference in `doc/add-item` and the shared rules in `DESIGN_SYSTEM.md`. The 2026-10-18 request adds system icons to Installed apps rows.

**Animation.** Short and few, because Orbit must stay fast.
- Showing and hiding the launcher: none. It is instant.
- Page switch: the adjacent page appears beside the open page during a mouse drag, and both pages slide together after release with ease-out motion. A full switch takes about 170 ms; a partly dragged page settles faster according to the distance left.
- Folder open and close: the panel grows from its tile and shrinks back to it, about 220 ms. Its content fades in as it opens.
- Drag: other items slide into the gap, about 120 ms, and the dropped item settles into place.
- Sheet and alert: the backdrop and panel fade in, about 120 ms.
- Focus ring and hover: none.
- Input never waits. Keys and clicks take effect immediately and the animation only follows. There is no animation setting in the MVP.

## 11. Saved files

One folder: `~/.config/orbit/` (respecting `XDG_CONFIG_HOME`).

| File | Holds |
|---|---|
| `config.json` | Version, pages, items, folders, pinned order and file and directory search paths |
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
2. **Data:** `config.json`, atomic writes, the empty launcher, pages with scrolling and the Add tile.
3. **Apps:** the Add item sheet for custom commands, launching, letter and SVG icons, the context menu, the delete alert.
4. **Folders and drag and drop:** folders, reordering, moving between pages, mouse page swipes, creating and deleting pages. This is the largest step.
5. **Item types and App sources:** Websites, files and directories; Actions and custom commands; scan, cache and choose installed apps from the Add item sheet.
6. **Dock:** Recent, Pinned and pin reordering.
7. **Keyboard:** the few keys in `SHORTCUTS.md` and the focus ring.
8. **Settings and looks:** startup page, appearance, background color, iOS polish.
9. **Glyph picker:** the Phosphor Icon picker.
10. **Packaging:** the `PKGBUILD` and the README.

## 14. Assumptions to confirm

1. Item size: Use 80 px icons, 14 px labels and 144 px spacing between item centers. Size controls remain deferred.
2. Editing an installed app changes its name and icon, not what it launches.
3. If an installed app is gone from the Installed apps list after a refresh, its item stays on its page. Launching it shows a banner such as "Couldn't start 'X': not found".

## 15. Open questions

None at the moment.
