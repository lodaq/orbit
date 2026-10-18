# Orbit shortcuts

Orbit is made for the mouse. Keys are few and fixed: they are not editable and there is no Shortcuts section in Settings. Ctrl+K focuses Search anything; there are no bare letter or digit shortcuts. Everything a key can do can also be done with the mouse (see the mouse table below and `SPEC.md` section 6).

## Launcher

The supplied UI update adds Ctrl+K to focus Search anything and Enter to open its first result. Esc clears search before hiding the launcher. Grid focus navigation remains deferred.

| Action | Shortcut |
|---|---|
| Previous page (outside fields, sheets and folders) | `←` |
| Move focus down | `↓` |
| Move focus up | `↑` |
| Next page (outside fields, sheets and folders) | `→` |
| Open (launch an app, open a Website, File or Directory, open a Folder, activate the Add tile) | `Enter` |
| Close the top layer (picker, then sheet or alert, then open folder, then launcher; on Settings, go back) | `Esc` |

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
- In a Name, URL or Command field: it moves to the next field.

## Mouse (fixed)

| Mouse | Result |
|---|---|
| Click an item | Launch an app, open a Website, File or Directory, or open a Folder |
| Click the Add tile | Open the Add item sheet |
| Click a System action | Add it as an App |
| Click an installed app | Add it to the current page |
| Choose a file or directory in its Add item sheet | Add an item named after the selected path |
| Right-click an item | Context menu: Edit, Pin or Unpin, Delete |
| Drag an item | Move it between pages, into a folder, or reorder it |
| Wheel up / down | Scroll the page |
| Shift + wheel up | Next page |
| Shift + wheel down | Previous page |
| Click a page dot | Open that page |
| Drag an empty area of the page left or right | Open the previous or next page |
| Click Add page in Settings | Add an empty page |
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
