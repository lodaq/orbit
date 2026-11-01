# Orbit

Orbit is a desktop app-launcher replacement for Windows and Linux (Arch Linux with Hyprland), written in Rust. It replaces the desktop launcher and takes over the jobs that would otherwise send you to your browser bookmarks, your file search or your task app.

## What Orbit does

**App-launcher replacement.** A fullscreen Launcher with a spatial Grid: Items sit on Pages in exact cells, Folders group them, Widgets take more room, and a Dock at the bottom keeps Pinned and Recent Items one click away. Drag an Item to the cell you want it in.

**Bookmark replacement.** Websites are Items on your Pages. The icon is fetched once and saved, so each one opens in your browser with a single click.

**File search replacement.** Point Orbit at the folders you care about. It indexes them on the first search and keeps the index until a path changes.

**Directory search replacement.** Point Orbit at the roots that hold your folders. Their immediate Directories appear in Search anything, next to everything else.

**Task manager replacement.** A TickTick Widget lives on a Page with Inbox, Today and Next 7 Days views, and `t` in Search anything creates a task with a due date, a priority, a tag and a list.

**Internet Usage widget.** Add it from Add item → Widgets to show today's download, upload and total traffic from the running vnStat daemon. It refreshes every minute; vnStat's local database can make the displayed counters lag by a few minutes. The widget lives inside Orbit's fullscreen window, so use Auto or Place on a Home Screen page instead of a separate Hyprland or Sway window rule.

**Search Prefix routing.** Search anything finds saved Items, System actions, the Installed apps list, files and Directories. Type a Token, press Space or Tab, and Orbit sends the rest of the query to the destination you picked: YouTube, ChatGPT, Claude, Perplexity, Wikipedia, GitHub, Google Maps, Image Search, Translate and TickTick, plus the search engines Brave, Google, DuckDuckGo, Qwant, Bing, Startpage and Ecosia. You type the query in Orbit instead of the browser's search field. With no results, Enter searches the web in Brave. Prefixes and Actions are editable in Settings, and you can add your own URL and shell Actions.

**System actions.** Lock screen, Take screenshot, Sleep, Log out, volume, mute and media playback are Items on your Pages.

## Prerequisites and build

Install the vnStat package and keep its daemon running to use Internet Usage. Build Orbit with `cargo build --release`. Start the hidden launcher process with `cargo run --release -- daemon`, then open it with `cargo run --release -- toggle`.

## Speed

Orbit is Rust and Slint: one binary, no web view. A resident Daemon starts at login, keeps the Pages, icons and search index in memory, and waits for `orbit toggle`, so showing the Launcher does no disk or network work and stays well inside the 50 ms budget.

Showing and hiding the Launcher is instant and is never animated. The animations are on what happens inside it: Page transitions, a Folder opening and closing, Items moving into the gap while you drag, and the fade of a Sheet or an Alert.

## Using it

Everything works with the mouse. The keyboard repeats what the mouse can do: Ctrl+K focuses Search anything, Enter opens the selected result, Esc closes the top layer, and the arrows move between Items and Pages. Every binding can be changed in Settings → Shortcuts.
