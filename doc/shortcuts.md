# V2 authority

The user approved `Orbit_Desktop_Launcher_UX_Spec_v2.md` on 2026-10-20 as authoritative wherever it conflicts with this document. Windows and Arch Linux, spatial grids, widgets, exclusive dock ownership, new UI terms, and the complete v2 build sequence are authorized. Existing visual artwork and nonconflicting performance and coding rules remain in force. The older build-step restriction does not apply to v2.

# Orbit shortcuts

Orbit is mouse first. Settings → Shortcuts lists Orbit bindings and lets you add or delete them. To change a binding, use its ×, confirm with ✓, then use Add to record a replacement. Chip labels do not start recording. Choose an action and record a key combination; duplicate combinations are rejected. The tables below are defaults. Text editing shortcuts are provided by text fields, and the global toggle remains in Hyprland. Ctrl+K returns focus to the search field; while results are visible, arrows navigate results and typing edits the query.

## Launcher

The launcher opens with Search anything focused. Ctrl+K focuses it again and Enter opens its selected result; when no local items match, search engines appear with the default first and selected; Enter searches using the selected engine. Esc clears search before hiding the launcher. Grid arrows move between spatial neighbors, Enter opens the focused item, Shift+F10 opens its Context menu, and PageUp/PageDown switches pages.

| Action | Shortcut |
|---|---|
| Move left in search results / Previous page outside search | `←` |
| Move focus down | `↓` |
| Move focus up | `↑` |
| Move right in search results / Next page outside search | `→` |
| Open (launch an app, open a Website, File or Directory, open a Folder) | `Enter` |
| Close the top layer (picker, then sheet or alert, then open folder, then launcher; on Settings, go back) | `Esc` |

Actions with no default binding can also be assigned: Open Settings, Add page and Delete page. Deleting the final binding disables an Orbit action’s shortcut; its mouse control remains available.

## Sheets, pickers, alerts and Settings

| Action | Shortcut |
|---|---|
| Next field or button | `Tab` |
| Previous field or button | `Shift+Tab` |
| Highlight the next item in a picker list | `↓` |
| Highlight the previous item in a picker list | `↑` |
| Choose | `Enter` |
| Close the top layer | `Esc` |

**Choose** does one thing, depending on where the focus is:
- In a picker list or a filter box: it chooses the highlighted item.
- On a button (Save, Cancel, Delete) or a Settings control: it presses it. In an alert, Cancel is focused first.
- In a Name, URL, Path or Command field: it moves to the next field or saves the item.

## Desktop navigation

Type a Prefix Token followed by Tab or Space to activate it. The Prefix name appears as a badge and search results are hidden until the Prefix is cleared. Backspace removes the active Prefix when the query is empty. Enter sends the remaining text to its Action. For a TickTick task, type `t`, press Tab or Space, then enter `Buy milk !today !high #home ~Buy` and press Enter. `!today` sets today's due date; `!high`, `!medium`, `!low` and `!none` set priority; `#tag` adds a tag; `~` and `^` select a list. Put a multiword list name at the end, for example `~hall of fame`. Leading emoji may be omitted when the readable name identifies one list. Omit the list to use Inbox. Task creation updates the widget immediately, then confirms the save or rolls back with a specific error.

Search engines: `:br` Brave, `:g` Google, `:d` DuckDuckGo, `:q` Qwant, `:b` Bing, `:sp` Startpage and `:e` Ecosia. Other site-search Tokens are `g` or `ل` for ChatGPT, `c` or `ؤ` for Claude, `gh` for GitHub, `m` for Google Maps, `i` or `ه` for Image Search, `p` or `ح` for Perplexity, `w` or `ص` for Wikipedia, `ف` for Arabic-to-English Translate, `tr` for English-to-Arabic Translate, and `y` or `غ` for YouTube. Type a Token, press Space or Tab, enter the query, then press Enter.

- `←`, `→`, `↑`, `↓`: move focus to the nearest item in that direction.
- `Enter`: open the focused item.
- `Shift+F10`: open its Context menu.
- `PageUp`, `PageDown`: change Page.
- Context menu → Move to Page transfers an item to a selected Page.

## Mouse (fixed)

| Mouse | Result |
|---|---|
| Click an item | Launch an app, open a Website, File or Directory, or open a Folder |
| Right-click empty page space → Add item | Open the Add item sheet |
| Click a System action | Add it as an App |
| Click an installed app | Add it to the current page |
| Choose a file or directory in its Add item sheet | Fill Path; Save adds an item named after the path |
| Right-click a page item | Context menu: Edit, Pin or Unpin, Delete |
| Right-click a pinned dock item | Unpin |
| Right-click a Recent dock item | Remove from Recent |
| Drag an item | Move it between pages, into a folder, or reorder it |
| Wheel up / down | Scroll the page |
| Shift + wheel up | Next page |
| Shift + wheel down | Previous page |
| Click a page dot | Open that page |
| Click + beside the page dots | Add and open a page |
| Drag an empty area of the page left or right | Open the previous or next page |
| Click Add page in the empty-space Context menu | Add an empty page |
| Add, Edit or Delete a file or directory search path in Settings | Manage folders searched by Search anything |
| Click the Settings icon | Open Settings |
| Click Back in Settings | Return to the launcher |
| Click the Close icon | Hide the launcher |
| Click the backdrop | Close the top layer |

## Rules

1. **Only the top layer gets keys.** An alert gets them before a sheet, a sheet before an open folder, an open folder before the launcher.
2. **Open folder.** The arrow keys and `Enter` act on the folder's apps.
3. **Text fields.** In a text field the arrow keys move the text cursor. `Esc` still closes the sheet.
4. **Edges stop, they do not wrap.** An arrow key at the edge of a grid does nothing.
5. **The dock is the last row.** `↓` from the bottom row of a page enters the dock, `↑` goes back up, `←` and `→` walk across the separator.
6. **A drag in progress** is cancelled by `Esc`.
7. **The focus ring is for keyboard use.** It is hidden when the launcher opens and appears when you press a key. Using the mouse hides it again.
8. **Toggle Orbit is not a key in Orbit.** The toggle key lives in `hyprland.conf`.
9. **Any keyboard layout.** All keys here work the same on every layout.

Search anything: **Ctrl+W** deletes the previous word using native text editing, including selected text and undo. Manage or remove **Delete previous word** in Settings → Shortcuts. It acts only while Search anything is focused with no Settings or layer open. Existing bindings are preserved; an already assigned Ctrl+W is not replaced.

The top-bar Lock layout / Unlock layout button has no default shortcut. While locked, Add page and Delete page shortcuts are disabled; page navigation remains available.

English assistance: **Accept next word** uses Ctrl+E by default while Search anything has a visible next-word prediction at the end of the query. It is configurable in Settings → Shortcuts. Existing Ctrl+E assignments are preserved; Tab also accepts a visible prediction; an exact Prefix Token takes priority.
