# Orbit glossary

One word for one thing. If a word is not in this file, do not use it in the UI, the code or the docs.

## The app

| Term | Meaning |
|---|---|
| **Orbit** | The application. |
| **Launcher** | The fullscreen view with pages and the dock. Shown by `orbit toggle`. Hidden by launching an app, pressing Esc, toggling again, losing focus, or the Close button. It always opens with all folders closed. |
| **Daemon** | The background process that starts at login and keeps Orbit ready, so showing the launcher is instant. |
| **Settings** | The dark panel with Add page, Refresh installed apps and Back. Further settings remain deferred. The Back button or Esc returns to the launcher. |
| **Background** | The radial gradient behind the pages, from the supplied SVG reference. |
| **Top bar** | The row at the top of the launcher: the Orbit logo and name on the left, the Settings and Close icons on the right. |

## Things you arrange

| Term | Meaning |
|---|---|
| **Item** | An app, website, file, directory or folder. Anything that can sit on a page. |
| **Item type** | The kind of item: App, Website, File, Directory or Folder. |
| **App** | A tile that starts something. Has a **name**, an **icon** and a **launch**. It has no color. |
| **Launch** | How an app starts. Either a **custom command** or an **installed app**. |
| **App source** | One of the choices for adding an App: Actions, Installed apps or Custom command. |
| **Actions** | The list of predefined System actions available as Apps. |
| **Custom command** | An app whose launch is a command you typed (for example `firefox --private-window`). Run directly, without a shell. |
| **System action** | A predefined operation such as Lock screen, Take screenshot, Sleep or Log out. It is saved as an App. |
| **Lock screen** | A System action that locks the current session. |
| **Take screenshot** | A System action that saves a screenshot in `~/Pictures/Orbit/`. |
| **Sleep** | A System action that suspends the computer. |
| **Log out** | A System action that ends the Hyprland session. |
| **Installed app** | An app whose launch is a reference to a program found on this computer. It stores no command. Added from the Installed apps list. The same installed app can be added more than once. |
| **Installed apps list** | The saved index of programs found on this computer. Changed only by **Refresh installed apps**. Never updates by itself. |
| **Refresh installed apps** | Scans the application entries installed on this computer and updates the Installed apps list. |
| **Website** | Opens a URL in the default browser. Its icon is fetched once when added and saved in Orbit's icons folder. |
| **URL** | The web address a Website opens. |
| **File** | An item made by choosing a file. Its name comes from the file and it opens with the default app. |
| **Directory** | An item made by choosing a directory. Its name comes from the directory and it opens in the default file manager. |
| **Choose file** | Opens the native file chooser to select a file for an item. |
| **Choose directory** | Opens the native file chooser to select a directory for an item. |
| **Path** | The location of a file or directory on this computer. |
| **Folder** | Holds apps. Opens like an iOS folder. Cannot contain other folders. Its tile shows a small preview of its apps. |
| **Page** | One screen of items, like a phone home screen page. Pages have no names. Maximum 9. An empty page is deleted when you leave it, unless it is the only one. |
| **Page swipe** | Dragging an empty area of a page left or right to open the previous or next page. |
| **Grid** | The rows and columns of tiles on a page or inside an open folder. |
| **Tile** | The rounded square that shows one item (an app, website, file, directory or folder) or the Add tile. |
| **Page dots** | The row of dots that shows which page is open. |
| **Add tile** | The "+" tile at the end of a page. Opens the Add item sheet. |

## The dock

| Term | Meaning |
|---|---|
| **Dock** | The semi-transparent bar at the bottom, styled like the macOS Dock. 48 px icons with names below and a separator between Recent and Pinned. |
| **Recent** | Successfully opened items, newest first. The two newest unpinned Apps, Websites, Files or Directories appear on the left side of the dock; folders are excluded. |
| **Pinned** | Items you chose to show in the dock. Pinning is a flag: the item also stays on its page. Displayed in saved pin order; drag reordering is deferred. |
| **Separator** | The thin vertical line between Recent and Pinned when both are present. |

## Icons

| Term | Meaning |
|---|---|
| **Icon** | The picture on an App, Website, File or Directory tile. |
| **Icon picker** | The panel where you choose an App, File or Directory icon. The only place the glyph library is loaded. |
| **Letter icon** | The first letter of the name. The default. Also called a monogram. |
| **Glyph** | An icon chosen from the built-in library (Phosphor). Saved as SVG, so the library is not needed afterwards. |
| **SVG icon** | SVG text you paste in. |
| **System icon** | The icon of an installed app, resolved once by Refresh installed apps and saved in Orbit's icons folder. |

## Things that appear on screen

| Term | Meaning |
|---|---|
| **Context menu** | The small menu that appears when you right-click an item: Edit, Pin (or Unpin), Delete. |
| **Sheet** | A form that appears over the launcher with a dimmed backdrop behind it: Add item, Edit app, Edit website, Edit file, Edit directory or Edit folder. (Developers also call these *modals*. In Orbit they are always called sheets.) |
| **Alert** | A small confirmation box with two buttons. Used for Delete. |
| **Panel** | The rounded box that holds the content of a sheet, an alert, the Icon picker or an open folder. |
| **Layer** | Anything drawn over the launcher: an Icon picker, a sheet, an alert or an open folder. The **top layer** is the one drawn last. It is the only one that gets keys and the one Esc closes. |
| **Backdrop** | The dimmed layer behind a sheet, an alert or an open folder. Clicking it closes the top layer. |
| **Segmented control** | A row of two or more connected buttons where exactly one is chosen, such as App or Website in the Add item sheet. |
| **Stepper** | A pair of minus and plus buttons that change a number, such as the font size in Settings. |
| **Focus ring** | The outline drawn around the item the keyboard is on. |
| **Tooltip** | The name shown when you hover or focus a dock icon. |
| **Filter box** | The small text field at the top of the Icon picker and of the Installed apps list. It narrows the list as you type. The separate Search anything field searches page items. |
| **Banner** | A short inline error, for example when a launch fails. The launcher stays open. |

## Keys and mouse

| Term | Meaning |
|---|---|
| **Shortcut** | A key (or key combination) assigned to an action. Fixed, listed in `SHORTCUTS.md`. The only word used for this, in the UI, the code and the docs. |
| **Action** | Something a shortcut can do, such as "Open" or "Close the top layer". |
| **Mouse gesture** | Wheel, Shift+wheel, right-click and drag. Not shortcuts. |

## Words we no longer use

| Old word | Use instead |
|---|---|
| Slide, slice | Page |
| Group | Folder |
| Keybinding, key binding, hotkey | Shortcut |
| Modal, model, dialog, popup | Sheet (forms), Alert (confirmations), Context menu (right-click) |
| Generated app | Installed app |
| Update apps list | Refresh installed apps |
| Browse | Installed apps list |
| Close (as the name of the action) | Close the top layer |
| Color (on an app or folder) | Removed |
| Customize | Removed |
| Image icon, background image | Not in the MVP (see `PLANNED_FEATURES.md`) |

## Supplied UI terms

| Term | Meaning |
|---|---|
| **Search anything** | Searches saved page items by name across pages. Ctrl+K focuses the field. |
| **No items found** | Empty search results. |
| **Add page** | Creates an empty page from Settings. |
| **Switch pages** | Footer hint for Left and Right. |
| **Back to desktop** | Footer hint for Esc. |
