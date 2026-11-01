# V2 authority

The user approved `Orbit_Desktop_Launcher_UX_Spec_v2.md` on 2026-10-20 as authoritative wherever it conflicts with this document. Windows and Arch Linux, spatial grids, widgets, exclusive dock ownership, new UI terms, and the complete v2 build sequence are authorized. Existing visual artwork and nonconflicting performance and coding rules remain in force. The older build-step restriction does not apply to v2.

# Orbit spec

## Current behavior

Remembrances combines morning and evening Adhkar in one widget. It follows the first Azan Reminder's saved schedule and time zone: Fajr → Dhuhr selects morning, Dhuhr → next Fajr selects evening. Recommended windows are Fajr → Sunrise and Asr → Maghrib. Mode progress stays separate; evening continues through midnight. The gallery offers one Remembrances card; existing morning/evening pairs on each page combine without discarding saved progress. Prayer times come from the existing explicit Azan Fetch; no additional network requests run. See `widgets.md` for migration and missing/stale schedule behavior.

Settings → Data follows the supplied Data design: a storage meter with actual local sizes, grouped collection navigation with total counts, a scrollable entry list with attribution below content, and a selected-row editor beside the list. Narrow layouts stack the editor below the list. Search submits with Enter. Add entry and Edit preserve the full content and display character counts; errors appear inside the editor. Delete requires confirmation in the selected row. Pagination stays below the list. Achievements history is read-only.

Settings → Data uses one SQLite file, `widgets.db`, with separate tables for Quotes, Facts, Motivation, Affirmations, Vocabulary, Quran Player, Azan Reminder, Habits and Achievements. Version 1 content migrates transactionally with its row IDs preserved. Original starter content is added once. All nine local cards appear in Add item → Widgets with Auto and Place. Content rotates locally and daily selections survive restart; vocabulary advances on launcher opening. Habits, affirmations and learned words update achievements with typed finished IDs, and completed TickTick IDs are recorded under their completion dates, with task titles retained for Achievements labels. Achievements shows today with a completion count, date and scrollable checked rows; Data holds the history. Quran audio is imported into Orbit's audio folder and opens in the default audio application. Azan displays the latest saved schedule and warns if stale. API candidate Fetch, desktop reminders and embedded audio controls remain planned. Existing launcher backups do not include the widget database.

`orbit ticktick` shows the launcher with an empty query and the TickTick Prefix active, including when the launcher is already visible. The personal Hyprland Alt+T binding runs this command for quick task entry.

Search results show the two immediate parent directories of a file or directory, or the URL or custom command, beneath the name and source. Hovering a result or selecting it with arrow keys shows the full location in a wider detail strip above the dock.

Multiline fields scroll vertically when their content exceeds the visible area, without a scrollbar, and automatically keep the text cursor visible. This applies to task notes and descriptions, commands, SVG markup, Action descriptions and scripts, and model prompts.

Single-line fields scroll horizontally when their text exceeds the visible width, without a scrollbar, and automatically keep the text cursor visible. This includes task titles, names, URLs, paths, filters and credential fields.

Search anything excludes Folder tiles and searches their contained Apps, Websites, Files and Directories by name. Matching folder items open directly from search results.

Settings → Search bar → General includes App search paths. Each path is an absolute directory or executable. Refresh adds executable files immediately inside configured directories (no recursion), plus explicitly configured executable files, to the cached Installed apps list. Paths persist in config.json; changing them does not scan until Refresh. On Linux, executable symlinks retain their link name and launch path. Missing executable files, broken symlinks and non-executable files are omitted on refresh. Existing desktop discovery remains enabled. Refresh skips desktop entries whose executable is missing or not executable; command names are resolved through PATH.

Overlapping folder targets accept Apps, Websites, Files and Directories. After a 500 ms tile-overlap dwell, release creates a folder or appends to an existing folder. Existing page items remain fixed during dragging and dropping. Occupied targets that cannot form a folder reject placement instead of displacing neighbors; empty-space drops move only the dragged item. Folder children retain shortcut type, URL/path, ID and icon through extraction, dissolution, unpacking and restart. Widgets and folders never become folder children.

`Orbit_Desktop_Launcher_UX_Spec_v2.md` is the current product specification and acceptance-test source. Its desktop Android-inspired spatial model supersedes the earlier list-only home-screen, Arch-only target and ordered-dock assumptions below. Existing Orbit appearance, item launching, search, native pickers and background-work requirements remain in force when compatible. New layout data lives in `config.json` version 3 with stable page IDs, explicit pixel-cell item rectangles, widget instances and active page; versions 1, 2 and 3 migrate on load while preserving rendered sizes. Grid item sizing is persisted in the existing Settings page. A dock pin owns a shortcut's layout location; page arrays retain its item record for existing app/search/recent lookups.



Every decision for the MVP, in one place. Words follow `glossary.md`. Shortcuts are in `shortcuts.md`. Coding rules are in `rules.md`. What is not in the MVP is in `planned-features.md` and `roadmap.md`.

## 1. What Orbit is

Orbit is a fullscreen desktop app-launcher replacement for Windows and Arch Linux, styled like iOS. It replaces the desktop launcher and takes over the work that would otherwise send you to your browser bookmarks, your file search or your task app. You arrange apps, websites, files, directories, folders and widgets on pages, like a phone home screen, and open it instantly with a key. It starts empty. Search anything searches saved page items, System actions, the cached Installed apps list, files in File search paths and immediate directories in Directory search paths by name. Ctrl+K focuses it. A Prefix routes the rest of the query to a site, a search engine, TickTick or a user Action, so the query is typed in Orbit and not in the browser's search field. Orbit is written in Rust with Slint and runs as one binary with a resident daemon that keeps everything ready, so showing the launcher does no disk or network work. Interactions and transitions are animated; showing and hiding the launcher is immediate.

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
- An Action is a named, reusable command with a description. Built-in Actions include URL templates and the TickTick task action; users can add shell scripts in Settings. A Prefix has a Name, Token and selected Action. Tab or Space activates a matching Token, hides search results and displays the Prefix name as a badge. Enter sends the remaining Search anything text to its Action.
- A Website opens its URL in the default browser. When it is added or saved again without an icon, Orbit reads the page's declared icon, then tries the host favicon and Google's domain favicon. The downloaded icon is saved in `icons/`. Fetching runs on the worker and never happens at startup or when showing the launcher.
- A File opens its path with the default app. A Directory opens its path in the default file manager. Both accept a typed absolute Path or a path selected with the native file chooser; the item name comes from the saved file or directory path. The default icon is a file or directory image, and can be changed in the Icon picker.
- A separate Action item is not needed. The predefined System actions are Apps; use Custom command for commands outside the Actions list. Search Prefixes dispatch to named Actions without parsing action-specific arguments in the search bar.

## 5. The launcher screen

From top to bottom: top bar, the open page, page dots, dock.

- **Top bar:** the supplied Orbit logo and wordmark (`ui/assets/orbit.svg`) and name on the left. The Settings and Close icons on the right.
- **Page:** a full-width, full-height page area below the top bar and above the dock, with a grid of items. Right-click empty page space and choose Add item to add an item. It scrolls vertically and has no item limit.
- **Page dots:** show which page is open. Clicking a dot opens that page. A + beside the dots adds a page, as does Add page in the empty-space Context menu.
- **Dock:** a floating, semi-transparent rounded bar at the bottom center, with no blur. Up to five recent items appear on the left and up to ten pinned items in saved order on the right, separated by a thin line when both sections have items. Each 48 px icon has no name: hovering one magnifies it and its neighbours, widens the bar by the room they take, as the macOS dock does, and shows the item name after 50 ms in a white rounded tooltip with a downward caret. The dock is always shown, even when it is empty. When its contents exceed the viewport, the dock scrolls horizontally only, with transparent arrows layered over the first and last visible items; the bottom position line is omitted.
- **On open:** the launcher opens on its current page with folders, Settings, sheets and alerts closed, search cleared, and Search anything focused. Grid focus navigation remains deferred.

## 6. Behavior

**Opening items.** Click, or `Enter` on the focused item.
- An app starts directly with no shell: the command is split into arguments and run detached, so the app survives a daemon restart. On success the launcher hides and the item enters Recent.
- A Website opens its URL in the default browser. A File opens with its default app. A Directory opens in the default file manager. These use `xdg-open` with the URL or path as one argument, without a shell. On success the launcher hides.
- On failure the launcher stays open and shows a banner such as "Couldn't start 'figma': not found".
- A folder opens instead of launching. Right-click empty page space → Add item opens the Add item sheet.

**Context menu:** right-click a page item for Edit, Pin (or Unpin), Move to Page and Delete. Delete asks for confirmation and includes widgets. Right-click empty page space for Add item, Add page or Delete page; page deletion asks for confirmation and is unavailable on the final page. A pinned dock item offers Unpin; a Recent dock item offers Remove from Recent.

**Sheets.** All open over a dimmed backdrop inside the launcher window. The Add item sheet fits the selected tab's content, has a maximum height of 760 px, and scrolls only when that content exceeds the available height.
- *Add item:* segmented control App, Website, File, Directory or Folder. For an App, a second segmented control: Custom command (first and selected by default), Actions or Installed apps. Actions shows Lock screen, Take screenshot, Sleep, Log out, Volume up, Volume down, Mute, Mute microphone, Play or pause, Next track and Previous track; choosing one adds it to the current page and closes the sheet. Audio and media actions use `wpctl` and `playerctl`, following the [Hyprland media examples](https://wiki.hypr.land/0.53.0/Configuring/Binds/) and [WirePlumber command reference](https://pipewire.pages.freedesktop.org/wireplumber/man/wpctl.html). Custom command has Name, Icon and Command, with Cancel and Save. Installed apps shows the saved Installed apps list and a Refresh button; clicking an app adds it right away at the end of the current page and closes the sheet, with no Save. Adding the same installed app again is allowed. Its name and icon come from the list and can be changed later with Edit. Website has Name and URL. File and Directory each have an editable Path, a native chooser button, a default Icon with Change icon beside it, and Cancel and Save. The chooser fills Path. Save checks the path, derives the item name, adds it to the current page and closes the sheet. A Folder has just a Name, with Cancel and Save.
- *Edit app:* Name, Icon, Command (custom command apps only). Cancel and Save. Delete is in the item's Context menu.
- *Edit website:* Name and URL. Its cached icon is refreshed when the URL is changed.
- *Edit file* and *Edit directory:* editable Path, native chooser button, Icon and Change icon, with Cancel and Save. Choosing another path fills Path; Save updates the item and its name from that path. The Icon preview and Change icon button are vertically centered together in these sheets and Edit app. Delete is in the item's Context menu.
- *Edit folder:* Name. Cancel and Save. Delete is in the item's Context menu.
- Keys inside sheets, pickers and alerts are listed in `shortcuts.md`. Their defaults can be changed in Settings. `Enter` in a Name, URL, Path or Command field moves to the next field or Save. Clicking the backdrop closes the sheet.

**Alert.** Delete always asks for confirmation.
- Deleting an app removes it everywhere: its page or folder, the dock.
- Deleting a Website, File or Directory removes that item from its page and the dock. It does not delete the file or directory from the computer. Deleting a Website also removes its cached icon.
- Deleting a folder deletes only the folder. Its apps move to the page where the folder was, at the folder's position. The alert says so, for example "Delete folder 'X'? Its 4 apps will move to the page."
- If the deleted folder was pinned, it is unpinned. A folder with no apps is simply removed.

**Pages.**
- No names. At least 1, at most 9.
- A new page is created by dropping an item past the right edge of the last page, clicking + beside the page dots, or clicking Add page in the empty-space Context menu.
- Drag an empty area of the page left or right to move to the previous or next page. The page follows the pointer without delay and settles into place after release. At the first or last page, it moves only a little and returns to the center.
- Widgets keep their full content visible on both pages throughout a page swipe and its settling transition.
- Two-finger horizontal touchpad scrolling changes pages in the scroll direction configured on the desktop. Small movements below 40 px do not change pages; momentum stays on the same page change until scrolling pauses for 350 ms. Vertical scrolling remains available inside widgets and pages.
- A 24 px gap separates page surfaces during a swipe.
- The page area fills the available width and height. The + beside the page dots creates and opens an empty page with the mouse.
- Leaving an empty page does not remove it. Delete page in the empty-space Context menu or the empty-space Context menu removes the current page after confirmation, moving its items to the previous page (or the next page when deleting the first). The last page cannot be deleted.
- First run: one empty page with Add item available from its empty-space Context menu.
- Startup page controls are deferred; reopening keeps the current page.

**Folders** work like iOS folders.
- They hold apps only and cannot nest. Websites, files and directories stay on pages and cannot be placed in folders. The tile shows a small preview of its apps.
- A folder opens from its tile into a centered panel over the backdrop, with one scrolling grid. Clicking outside or pressing `Esc` closes it.
- Dropping an app on another app makes a folder with both. Dropping an app on a folder adds it. Websites, files and directories can be reordered and moved between pages, but do not make folders.
- A folder disappears when its last app leaves it.
- Dragging an app out past the panel edge closes the folder visually and keeps the app under the pointer. Only releasing it over the page moves it out of the folder; Esc cancels without changing the folder.

**Dock.**
- Pin or Unpin in an item's context menu adds or removes it from the dock. The item stays on its page.
- Right-click a pinned dock item to Unpin it. Right-click a Recent dock item to remove it from Recent history.
- Recent tracks successfully opened Apps, Websites, Files and Directories, including Apps inside folders. Folders themselves do not enter Recent. The five newest unpinned items appear on the left; pinned items appear on the right in saved order, up to ten. An item appears only once in the dock.
- Recent keeps up to sixteen item IDs so older unpinned items can fill the left side when a newer item is pinned or deleted. Opening an item again moves it to the front. A successful launch writes `recent.json` only when the order changes.
- Clicking a dock item opens it, including a folder or an App inside a folder on another page. Dock drag reordering remains a later functional step.

**Drag and drop** (iOS behavior).
- A click without movement launches. Dragging makes the other items slide aside to open a gap, and dropping fills it.
- Holding an app over the central 112 px area of another app or folder for 2 seconds previews the combined folder on the target; releasing creates the folder or adds the app. The target stays in place and highlights while ready. A folder dropped onto anything only reorders.
- Holding a dragged item at the left or right screen edge advances through successive pages every half second until the edge is left or the last available page is reached. At the right edge of the last page, it shows a new empty page (if there are fewer than 9). Dropping inserts the item at the pointer's position and opens that page. Near the top or bottom edge, the page scrolls.
- Dropping on the dock snaps the item back. `Esc` during a drag cancels it. A drag changes saved data only on release. One stationary input surface tracks the pointer while rendered tiles move; the dragged tile follows without interpolation. Page and folder moves resolve their source and folder target by item ID. The same folder hold works after changing pages.

**Everything works with the mouse.** Orbit is mouse first, and the keyboard only repeats what the mouse can do.

| To do this | With the mouse |
|---|---|
| Open an item (launch an app, open a Website, File or Directory; open a Folder) | Click it |
| Add an item | Right-click empty page space → Add item |
| Edit, Pin or Unpin, Delete | Right-click the item, choose from the context menu |
| Reorder, move between pages, make a folder | Drag and drop |
| Change page | Click a page dot, or Shift + wheel |
| Add a page | Click + beside the page dots, or Add page in the empty-space Context menu |
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

Installed apps use their real system icon, resolved once when you press Refresh and saved into `icons/`.

## 8. Installed apps list

- A saved index of programs found on the computer. **Only the Refresh button changes it.** No automatic refresh, no file watcher, no scan at startup, no scan when a sheet opens.
- Read `.desktop` files from the XDG data folders (`/usr/share/applications`, `~/.local/share/applications`, Flatpak exports). Skip `NoDisplay`, `Hidden` and `Terminal=true` (terminal apps are not in the MVP). Strip field codes such as `%u` and `%F`. Store each app's start command in the list.
- Resolve each icon from the icon theme once, at refresh. This is the one place where a small crate is justified, because icon themes are hard to resolve by hand.
- Refresh shows a status such as "4 new apps found". It never adds anything to a page. You add apps from the list through the Add item sheet.
- Before the first refresh, the list is empty and shows a Refresh button.
- The list has a filter box that narrows it as you type. Clicking an app adds it to the current page immediately.

## 9. Settings

The top bar Settings control opens a dark panel with General, Shortcuts, Search bar, and Back. Add, Edit or Delete a path in either list. Each path is an existing folder and can be entered as an absolute Path. File search paths are searched recursively for files; Directory search paths show only their immediate directories. Indexing starts on the first non-empty search and stays cached until a path changes. The refresh status is visible in the panel. Esc returns to the launcher. Page margin and Item size controls save to config.json. Page margins are equal on all four sides, default to 10 px and allow 0–64 px. Page tiles stay square, default to 128 px and allow 80–240 px. Startup controls remain deferred.

Settings also has Export data and Import data. Export saves a versioned JSON backup containing pages, folders, saved icons, Pinned, Recent, search paths and the Installed apps list. Import opens the native chooser directly, then validates the chosen backup before replacing data. Cancelling the chooser leaves data unchanged. It retains the previous data in a sibling `orbit-before-import-<time>` directory and reloads the launcher. File and Directory items keep their paths; the referenced files and installed programs are not included. Both operations run on the worker and use a centered floating native file chooser. Orbit stays fullscreen behind the chooser, and returns focus to Settings when it closes. This explicitly requested addition is outside the original build order.

Settings has General, Shortcuts and Search bar tabs. Shortcuts lists each Orbit action once, with all assigned key combinations grouped in that row. Users can record, change, add and delete individual combinations. Duplicate key combinations are rejected. Bindings persist in config.json and backups; old configurations receive the defaults. Native text editing remains provided by text fields; the global toggle is configured in Hyprland. These user-requested changes extend the original fixed-shortcut scope.

## 10. Look

The authoritative visual reference is `doc/orbit-ui/orbit.png` and `orbit.svg` (1440 × 1080). The user approved this design replacing the previous light theme restrictions on 2026-10-18.

- Use the reference radial background, lavender artwork, logo, search field, Settings and Close controls, page dots, icon-only dock and footer hints.
- Embed Inter. Page labels are 14 px. Page icons are 92 px in the default 128 px tile, leaving a compact label gap; dock icons are 48 px.
- Top bar: logo at (48,40), 360 × 44 px centered search at y=34, controls 48 px apart at the right.
- Grid: 144 px between item centers at reference width (128 px item size + 16 px default Item Gap). The page area fills the available width and height, and the grid adds columns as width permits. The page and its 128 × 128 px item tiles have 1 px borders. Item tiles have a lighter background on hover. The gaps between tiles do not select items. Settings lets the user change Item Gap from 0 to 64 px.
- Page dots are centered 146 px above the bottom. The dock is 64 px tall, 22 px above the bottom, with rounded corners and 68 px per item. Hovering grows the icon under the pointer to 1.6× and its neighbours by less with distance, and the icons slide apart and upward from the bar so they never overlap; the bar widens by the room they take. After hovering an item for 50 ms, a white rounded tooltip bubble with a downward caret shows the item's name above the icon. The dock can grow to 1400 px wide and scrolls horizontally when its Recent and Pinned items overflow. Side controls sit outside the scroll viewport. The separator divides Recent from Pinned when both are present.
- Search filters saved page items, all System actions, the cached Installed apps list, files in File search paths and immediate directories in Directory search paths by name. Results from saved page items appear after the other matches. The first result is initially selected. Arrow keys move the selection through the grid without wrapping and scroll it into view. Enter opens the selected result. Clicking any result opens it. Each highlight encloses the icon and labels with padding and a gap from neighboring results. Results show their saved or cached icon when available. Choosing an action runs it, and choosing an installed app launches it without adding it to a page. Esc clears search before hiding.
- Prefix tokens activate with Tab or Space, display the Prefix name as a badge and hide search results while active. Enter sends only the remaining query as the Action argument. When no local results match, Enter searches with the web search engine selected in Settings → Search bar → General (Perplexity by default). Prefixes, Actions and the selected engine are stored in config.json; one Action can have several editable Prefix Tokens, and URL Actions encode query text. Site-search defaults include ChatGPT (`g`, `ل`), Claude (`c`, `ؤ`), GitHub (`gh`), Google Maps (`m`), Image Search (`i`, `ه`), Perplexity (`p`, `ح`), Wikipedia (`w`, `ص`), Translate ar to en (`ف`), Translate en to ar (`tr`), YouTube (`y`, `غ`), plus the web search engines. TickTick task creation parses `!today` (or `!!`), `!high`, `!medium`, `!low`, `!none`, `#tag`, and `^List`/`~List` in its own handler; omitted lists use Inbox. Shell Actions receive the query as a separate process argument, never interpolated into script source.
- Left and Right switch pages when a text field, sheet or folder is not active. Ctrl+K focuses search.
- Preserve native, interactive controls. The exported `main.slint` is only a full-screen PNG and cannot be used as the application implementation.
- Sheets, the Context menu and Delete alerts follow the supplied Add item reference in `doc/add-item` and the shared rules in `design-system.md`. The 2026-10-18 request adds system icons to Installed apps rows.

**Animation.** Short and few, because Orbit must stay fast.
- Showing and hiding the launcher: none. It is instant.
- Page switch: the adjacent page appears beside the open page during a mouse drag, and both pages slide together after release with ease-out motion. A full switch takes about 170 ms; a partly dragged page settles faster according to the distance left.
- Folder open: instant, with no movement or fade. Folder close: the panel shrinks back to its tile in about 220 ms.
- Drag: other items slide into the gap, about 120 ms, and the dropped item settles into place.
- Sheet and alert: the backdrop and panel fade in, about 120 ms.
- Focus ring: none. Hover: page tiles lighten, dock icons magnify and slide apart over 80 ms with ease-out motion, and the hovered item's name appears in a tooltip after 50 ms.
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
2. **Data:** `config.json`, atomic writes, the empty launcher, pages with scrolling and the empty-space Add item menu.
3. **Apps:** the Add item sheet for custom commands, launching, letter and SVG icons, the context menu, the delete alert.
4. **Folders and drag and drop:** folders, reordering, moving between pages, mouse page swipes, creating and deleting pages. This is the largest step.
5. **Item types, App sources and Search Prefixes:** Websites, files and directories; Actions and custom commands; scan, cache and choose installed apps from the Add item sheet. Add reusable shell-script and URL-template Actions, editable Actions and Prefixes settings, generic token activation/dispatch, the TickTick task Action, and default site-search prefixes.
6. **Dock:** Recent, Pinned and pin reordering.
7. **Keyboard:** the few keys in `shortcuts.md` and the focus ring.
8. **Settings and looks:** startup page, appearance, background color, iOS polish.
9. **Glyph picker:** the Phosphor Icon picker.
10. **Packaging:** the `PKGBUILD` and the README.

## 14. Assumptions to confirm

1. Item size: Use 92 px icons in the default 128 px tile, 14 px labels and 144 px spacing between item centers. Settings can change square page tiles from 80 to 240 px; icons scale to fit.
2. Editing an installed app changes its name and icon, not what it launches.
3. If an installed app is gone from the Installed apps list after a refresh, its item stays on its page. Launching it shows a banner such as "Couldn't start 'X': not found".

## 15. Open questions

None at the moment.

Settings groups Home Screen sizing, TickTick account and data under General. Search bar contains General (search engine and search paths), Prefixes and Actions. Page creation, deletion and Move page left/right are in the empty-space Context menu. TickTick widgets use the raised surface background.

Search anything wraps long queries and grows vertically, with results below the field. Backspace removes an active Prefix when its query is empty. When local items match, show those results. If none match, show no search engine tiles; Enter still searches with the saved default engine (Perplexity initially). The default engine select beside Search anything opens the engine list.

Search engine results use locally bundled brand icons in rounded tiles.

## Smooth cursor

Settings → Search bar → General → Smooth cursor saves immediately and defaults to On, including existing config files. Search anything uses a gliding block and bounded trail without delaying text or search. It fades softly while idle, hides for selection and snaps on line changes and reopening. Hidden or unfocused cursors perform no idle work. Native editing, selection, clipboard, shortcuts and IME are preserved; IME composition and Off use the native caret.

Settings → Shortcuts includes Delete previous word (Ctrl+W by default), scoped to focused Search anything. Native word boundaries, selection and undo are preserved. Existing configs receive the default once when Ctrl+W is free; removing it keeps it removed.

Shortcut displays use shared native Kbd keycaps, including search, footer, Settings bindings and recording. Search and close/page hints reflect saved bindings. Search anything reserves full cursor space at wrapped line ends and scrolls the entire final block into view, with or without a Prefix.

Search anything uses 16 px text for the query and placeholder. Opening a folder animates its panel from the tile to the centered overlay over 220 ms while the backdrop fades in; closing reverses the movement.


### Settings redesign and file-search exclusions (2026-10-22)

Settings follows `settings/DESIGN.md` with General, Shortcuts, Search bar and Back in a sidebar, and General, Prefixes and Actions in Search bar’s segmented control. The top bar, dock and bottom hints retain their existing appearance. Existing actions, sizing preview, Save, Reset and unsaved-sizing cancellation remain available. Shortcuts use display-only key chips with × and confirmation; use Add after removing a binding to change it.

Settings → Appearance lets the user save a solid background color as `#RRGGBB`, choose a PNG, JPEG or SVG background image, and Reset background to restore the supplied radial SVG. The selected image is copied into Orbit's saved icon storage and decoded by the existing worker; showing the launcher does no file I/O or image decoding.

Appearance also saves a Background images folder, defaulting to `/home/sebaq/Pictures/aesthetic-wallpapers/images`. The image-change icon at the bottom right of the Home Screen selects a different PNG, JPEG or SVG from that folder; image reads and decoding stay on the worker.

Search bar → General includes Excluded from file search. `config.json` version 3 adds `file_search_exclusions`, defaulting when missing to `.git`, `node_modules`, `.*`, `.local`, `.claude`, `.agent`, `.cache`, `.venv`, `__pycache__`, `dist`, `build`, `.next`, `target`, `.idea`, `.vscode`, `.npm`. An explicitly empty list stays empty. Trim entries and ignore blanks and duplicates; successful edits save atomically and invalidate the worker’s cached path index. Errors preserve the draft and saved list.

Patterns match whole descendant file or directory names case-sensitively. Only `*` is special and matches zero or more characters; `.*` matches dot-prefixed names. Matching directories are not descended into. Explicitly configured search roots are still entered. Directory search and saved launcher items are unaffected. Index rebuilding stays on the worker and does no work when showing Orbit.


## Layout lock (2026-10-22)

Lock layout / Unlock layout is the padlock immediately left of Settings. One saved state applies to all pages, folder contents and the pinned dock; existing configurations default to unlocked. While locked, disable dragging, resizing, Move to Page, pin/unpin, adding, duplicating, deleting and reordering items/pages, layout sizing controls and Import data. Rust handlers reject these operations as well as the interface. Navigation, launching, search, scrolling, Recent updates, widget interactions and editing existing content remain available. Locking cancels pending drag/resize and placement previews. The lock survives hiding and restarting Orbit; no default shortcut is added.

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

AI mode uses a dedicated opaque answer card aligned with a wider prompt field. While its Prefix is active, page items/widgets, page indicators, search results/details, the search-engine chooser and page-switch hints are hidden, including for an empty prompt. Page navigation and hidden-result activation are disabled; the dock stays available. Leaving AI mode restores the normal launcher. Empty/loading/error content stays compact; answers grow to a bounded height and scroll with native selection and Copy.

AI view verification (2026-10-24): the rebuilt release was opened on a populated page with TickTick widgets. Both Space and Tab activated the clean AI view; a visible local-model answer and exact clipboard copy were checked, a 50-line reply scrolled through its final line, arrow-key editing worked, and Esc restored the populated page. Formatting, Clippy and all 91 enabled tests passed (6 existing opt-in tests ignored). Captures and details: target/ui-check/ai-clean-*.

UI update (2026-10-25): Add item → Widgets is a responsive card grid. Footer shortcut hints are removed. The dock fits its items with equal left and right padding; an empty dock remains visible. Shared Theme tokens govern repeated type, corner and feedback values across the interface.

Settings → Search bar → General groups App search paths, File search paths, Directory search paths and Excluded from file search in one Search locations panel. Refresh rebuilds Installed apps and rescans all configured file and directory paths on the worker, applying file exclusions. All installed-app refresh controls use this shared operation. Newly created files become searchable after Refresh without restarting Orbit.

Search results for saved shortcuts, System actions, Installed apps, Files and Directories offer Pin and Add to Page in their Context menu. Add to Page places the shortcut on the current Page. Dragging a result hides search results and English assistance while revealing the current Page; release on free page space saves its exact placement, and release on the dock pins it. Existing saved items move by ID instead of creating a duplicate; new results become saved shortcuts. Invalid drops and Esc cancel without saving. Search engine fallback results cannot be added. Lock layout prevents these changes. Successful placement clears Search anything.

Morning Remembrances (Adhkar), explicitly requested 2026-10-27, adds a local widget with the user's 25 Arabic entries. Each tap counts one repetition; reaching the required count plays a chime and advances automatically. Only completing every entry plays the distinct final chime and shows Restart. Skip retains unfinished entries, Previous preserves counts, and source details expand. Daily per-instance progress and editable content live in widgets.db (schema 3); the bundled source JSON remains unchanged. Add it through Add item → Widgets → Morning Remembrances → Auto or Place.

Evening Remembrances (Adhkar), requested 2026-10-27, adds the supplied 25 evening entries as a separate widget and Settings → Data collection. It reuses Morning Remembrances' fixed 80 px counter, text scrolling, source details, navigation, sounds and daily per-instance progress. The gallery offers Auto and Place with the same sizing. Database schema 4 seeds evening entries once without replacing morning content or progress. Both collections remain offline and independently editable. See `widgets.md` for acceptance checks.

Completing all repetitions in a Morning Remembrances or Evening Remembrances widget adds that collection to the daily Achievements, once per widget. Restarting preserves the earned completion.

Azan Reminder (2026-10-28) has a settings icon for per-widget City, Country and Time zone, defaulting to Sohag, Egypt and Africa/Cairo. Fetch times retrieves today's Aladhan method-5 schedule for review; Save accepts the preview locally. No automatic Fetch occurs. Named zones govern dates and countdowns, including seasonal clock changes. Missing or stale schedules withhold countdowns. Existing schedules remain in Settings → Data. The widget emphasizes the next prayer and time above aligned daily rows.

TickTick widget update (2026-10-28): the header picker scrolls through Today, Next 7 Days, all tasks, Lists and Tags. A tag selection filters tasks by the exact tag and persists per widget. Header add/menu controls use matching inset icon buttons. High, Medium and Low priorities use red, amber and blue checkboxes; no priority uses the neutral checkbox. Arabic-first titles use RTL row placement, right alignment and the embedded Arabic font. Wheel and trackpad scrolling uses a higher proportional scroll distance with no interpolation delay, so rapid wheel input accumulates immediately.

Focus task redesign (2026-10-28): the TickTick Focus Task card uses a wrapping task title, quiet list context, priority/date metadata, tags and readable notes/checklists. A circular priority-colored completion control stays at the upper right, and Open task opens the existing editor. Details reveals the full task metadata inside the same scrollable body. Short cards use a compact title layout. Arabic titles and notes keep their own text direction. Header dragging, resizing and immediate wheel scrolling remain available.

English assistance displays only the untyped completion suffix; corrections to typed letters use an explicit Autocomplete replacement preview. Ctrl+E, Tab and Accept word accept the same validated current-word or next-word edit. Grammar corrections remain separate clicks. Known built-in prompts migrate without replacing custom prompts; grammar defers the word still being typed. See `english-assistance.md` for the 2026-10-30 repair and verification.

English Suggestions show Ctrl+1/2/3 keycaps for direct application. Ctrl+N/J moves the highlight down, Ctrl+P/K moves it up, and Enter applies the selected rewrite without submitting. The selection resets on refreshed or invalidated results. Ctrl+K keeps its normal focus action outside current suggestions.
