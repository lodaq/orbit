# V2 authority

The user approved `Orbit_Desktop_Launcher_UX_Spec_v2.md` on 2026-10-20 as authoritative wherever it conflicts with this document. Windows and Arch Linux, spatial grids, widgets, exclusive dock ownership, new UI terms, and the complete v2 build sequence are authorized. Existing visual artwork and nonconflicting performance and coding rules remain in force. The older build-step restriction does not apply to v2.

# Orbit glossary

For v2 desktop layout terms, including Item Size, Item Gap, Grid Position, Widget Gallery, widget footprints, placement preview, insertion edge, folder center, Move to Page and Remove from Home, follow `Orbit_Desktop_Launcher_UX_Spec_v2.md`. These terms are approved alongside the terms below.

One word for one thing. If a word is not in this file, do not use it in the UI, the code or the docs.

## The app

| Term | Meaning |
|---|---|
| **Orbit** | The application. |
| **Launcher** | The fullscreen view with pages and the dock. Shown by `orbit toggle`. Hidden by launching an app, pressing Esc, toggling again, losing focus, or the Close button. It always opens with all folders closed. |
| **Daemon** | The background process that starts at login and keeps Orbit ready, so showing the launcher is instant. |
| **Settings** | The dark panel with General, Shortcuts, Search bar and Back. The Back button or Esc returns to the launcher. |
| **Background** | The radial gradient behind the pages, from the supplied SVG reference. |
| **Top bar** | The row at the top of the launcher: the Orbit logo and name on the left, the Settings and Close icons on the right. |

## Things you arrange

| Term | Meaning |
|---|---|
| **Item** | An app, website, file, directory or folder. Anything that can sit on a page. |
| **Item type** | The kind of item: App, Website, File, Directory or Folder. |
| **App** | A tile that starts something. Has a **name**, an **icon** and a **launch**. It has no color. |
| **Launch** | How an app starts: a **custom command**, an **installed app** or a **System action**. |
| **App source** | One of the choices for adding an App: Actions, Installed apps or Custom command. |
| **Actions** | The list of predefined System actions available as Apps. |
| **Custom command** | An app whose launch is a command you typed (for example `firefox --private-window`). Run directly, without a shell. |
| **System action** | A predefined operation such as Lock screen, Take screenshot, Sleep or Log out. It is saved as an App. |
| **Lock screen** | A System action that locks the current session. |
| **Take screenshot** | A System action that saves a screenshot in `~/Pictures/Orbit/`. |
| **Sleep** | A System action that suspends the computer. |
| **Log out** | A System action that ends the Hyprland session. |
| **Volume up** | A System action that raises the default audio output volume. |
| **Volume down** | A System action that lowers the default audio output volume. |
| **Mute** | A System action that toggles the default audio output mute state. |
| **Mute microphone** | A System action that toggles the default audio input mute state. |
| **Play or pause** | A System action that toggles media playback. |
| **Next track** | A System action that skips to the next media track. |
| **Previous track** | A System action that returns to the previous media track. |
| **Installed app** | An app whose launch is a reference to a program found on this computer. It stores no command. Added from the Installed apps list. The same installed app can be added more than once. |
| **Installed apps list** | The saved index of programs found on this computer. Changed only by **Refresh**. Never updates by itself. |
| **Refresh** | Updates the Installed apps list and rescans saved app, file and directory search paths, applying file exclusions. |
| **Website** | Opens a URL in the default browser. Its icon is fetched once when added and saved in Orbit's icons folder. |
| **URL** | The web address a Website opens. |
| **File** | An item made by entering or choosing a file Path, then saving. Its name comes from the file and it opens with the default app. |
| **Directory** | An item made by entering or choosing a directory Path, then saving. Its name comes from the directory and it opens in the default file manager. |
| **Choose file** | Opens the native file chooser and fills Path with the selected file. |
| **Choose directory** | Opens the native file chooser and fills Path with the selected directory. |
| **Path** | The editable location of a file or directory on this computer. |
| **File search path** | A folder searched recursively for files by Search anything. |
| **Directory search path** | A folder whose immediate directories appear in Search anything. |
| **Folder** | Holds launchable shortcuts: Apps, Websites, Files and Directories. Opens like an iOS folder. Cannot contain other folders. Its tile shows a small preview of its apps. |
| **Page** | One screen of items, like a phone home screen page. Pages have no names. Maximum 9. Leaving an empty page does not remove it. |
| **Page swipe** | Dragging an empty area of a page left or right to open the previous or next page. |
| **Grid** | The rows and columns of tiles on a page or inside an open folder. |
| **Tile** | The rounded square that shows one item (an app, website, file, directory or folder). |
| **Page dots** | The row of dots that shows which page is open, with a + control to add a page. |
| **Add item** | The command in the empty-page-space Context menu that opens the Add item sheet. New items use automatic free-space placement. |

## The dock

| Term | Meaning |
|---|---|
| **Dock** | The semi-transparent horizontal bar at the bottom. It shows 48 px icons with names, Recent and Pinned sections, and scrolls horizontally when its contents overflow. |
| **Recent** | Successfully opened items, newest first. Up to five newest unpinned Apps, Websites, Files or Directories appear on the left side of the dock; folders are excluded. |
| **Pinned** | Up to ten items you chose to show in the dock. Pinning is a flag: the item also stays on its page. Displayed in saved pin order. |
| **Separator** | The thin vertical line between Recent and Pinned when both are present. |

## Icons

| Term | Meaning |
|---|---|
| **Icon** | The picture on an App, Website, File or Directory tile. |
| **Icon picker** | The panel where you choose an App, File or Directory icon. The only place the glyph library is loaded. |
| **Letter icon** | One or two entered visible characters, including emojis. Existing automatic icons use the first letter of the name. |
| **Glyph** | An icon chosen from the built-in library (Phosphor). Saved as SVG, so the library is not needed afterwards. |
| **SVG icon** | SVG text you paste in. |
| **System icon** | The icon of an installed app, resolved once by Refresh and saved in Orbit's icons folder. |

## Things that appear on screen

| Term | Meaning |
|---|---|
| **Context menu** | The small menu that appears when you right-click an item or empty page space. Empty space offers Add item, Add page and Delete page; items offer their own actions, including Delete. |
| **Sheet** | A form that appears over the launcher with a dimmed backdrop behind it: Add item, Edit app, Edit website, Edit file, Edit directory or Edit folder. (Developers also call these *modals*. In Orbit they are always called sheets.) |
| **Alert** | A small confirmation box with two buttons. Used for Delete. |
| **Panel** | The rounded box that holds the content of a sheet, an alert, the Icon picker or an open folder. |
| **Layer** | Anything drawn over the launcher: an Icon picker, a sheet, an alert or an open folder. The **top layer** is the one drawn last. It is the only one that gets keys and the one Esc closes. |
| **Backdrop** | The dimmed layer behind a sheet, an alert or an open folder. Clicking it closes the top layer. |
| **Segmented control** | A row of two or more connected buttons where exactly one is chosen, such as App or Website in the Add item sheet. |
| **Stepper** | A pair of minus and plus buttons that change a number, such as the font size in Settings. |
| **Focus ring** | The outline drawn around the item the keyboard is on. |
| **Tooltip** | The name shown when you hover or focus a dock icon. |
| **Filter box** | The small text field at the top of the Icon picker and of the Installed apps list. It narrows the list as you type. Search anything searches page items, System actions, the cached Installed apps list and configured file and directory search paths. |
| **Banner** | A short inline error, for example when a launch fails. The launcher stays open. |
| **Toast** | A short floating message that confirms an action or reports a problem, then closes automatically. |

## Keys and mouse

| Term | Meaning |
|---|---|
| **Shortcut** | A key (or key combination) assigned to an action. Defaults are listed in `shortcuts.md`; editable in Settings. The only word used for this, in the UI, the code and the docs. |
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
| Update apps list | Refresh |
| Browse | Installed apps list |
| Close (as the name of the action) | Close the top layer |
| Color (on an app or folder) | Removed |
| Customize | Removed |
| Background image | Not in the MVP (see `planned-features.md`) |

## Supplied UI terms

| Term | Meaning |
|---|---|
| **Search anything** | Searches saved page items, System actions, the cached Installed apps list, files in File search paths and immediate directories in Directory search paths by name. Ctrl+K focuses the field. |
| **Search engine** | The website used for a web search when no local items match Search anything. Choose it in Settings → Search bar → General. |
| **Prefix** | A named Search anything rule with a Token and selected Action. Tab or Space activates a matching Token; Enter sends the remaining search text to its Action. |
| **Token** | The text that activates a Prefix in Search anything. |
| **Action** | A named reusable command that receives a Prefix query. Actions include built-in site searches, TickTick task creation, and user shell scripts. |
| **Action description** | Text in Settings that explains what an Action does and how to use it. |
| **Shell Action** | An Action that runs a user-provided shell script. It receives Search anything text as a separate argument, available as `$1` on Arch Linux and `$OrbitQuery` on Windows. |
| **URL Action** | An Action that opens a URL after replacing `%s` with safely encoded Search anything text. |
| **TickTick Action** | An Action that creates a TickTick task from the title, due date, priority, tags and list selectors in Search anything text. |
| **Prefixes** | Settings tab for creating, editing and selecting Prefix Actions. |
| **Actions** | Settings tab for creating and editing reusable Actions. |
| **Default search engine** | The search engine used when Enter is pressed with no local matches; the select beside Search anything opens the engine list. |
| **Add page** | Creates an empty page from the empty-space Context menu or the + beside the page dots. |
| **Switch pages** | Footer hint for Left and Right. |
| **Back to desktop** | Footer hint for Esc. |

| **Choose icon** | Opens the native file chooser for a saved icon image. |

## Saved data

| Term | Meaning |
|---|---|
| **Export data** | Save Orbit's data and icons to a backup file. |
| **Import data** | Restore Orbit's data and icons from a validated backup, retaining the previous data. |
| **Backup** | A versioned file containing Orbit's saved data and icons. |

| **Shortcuts** | Settings tab for listing, recording, changing and deleting action bindings. |
| **General** | Settings tab for Home Screen sizing, TickTick account and data. |
| **Data** | Settings tab for locally saved widget content and Orbit storage use. |
| **Content** | The saved text of a widget entry, including English, Arabic or mixed text. |
| **Attribution** | Optional author, source or details attached to a saved content entry. |
| **Entry** | One independently editable item of locally saved widget content. Duplicate content may have separate entry IDs. |
| **Quotes** | Local collection used by Random Quote. |
| **Facts** | Local collection used by Random Fact. |
| **Motivation** | Local collection of motivational content. |
| **Affirmations** | Local collection used by Affirmation. |
| **Vocabulary** | Local collection of words and their details. |
| **Delete page** | Remove the current page after confirmation and keep its items on a neighboring page. |

| **Page margin** | Equal space around each side of a page, adjustable in Settings. |
| **Item size** | The square page tile width and height in pixels, adjustable in Settings. |
| **Item Gap** | The spacing between page and folder tiles in pixels, adjustable from 0 to 64 in Settings. |

| **Search bar** | Settings tab containing General search settings, Prefixes and Actions. |

| **Smooth cursor** | Search bar setting for the animated block cursor in Search anything. On by default; Off restores the native text cursor. |

| **Delete previous word** | Configurable Search anything shortcut, Ctrl+W by default, that removes the word before the text cursor. |

| **Keycap** | A small rounded label displaying a keyboard key or shortcut. Implemented by the shared Kbd control. |

| **Excluded from file search** | Names or patterns skipped when indexing files under File search paths. Matching directory names also stop recursive descent; Directory search paths and saved items are unaffected. |

| **Lock layout** | The top-bar padlock that prevents changes to page, folder and pinned dock arrangement, additions, deletions and sizing. Its state is saved for all pages. |
| **Unlock layout** | The same padlock action that permits layout changes again. |

| **Automatic** | Website icon option that fetches and previews the URL's icon before Save. |
| **Custom icon** | Website icon option that uses the Letter, SVG or chosen image from the Icon picker. |
| **Fetched icon** | The preview of a Website icon retrieved automatically. |
| **Retry** | Fetch the Website icon again after a completed attempt. |

| **English assistance** | Search bar settings for local English suggestions, autocomplete and grammar feedback. |
| **Suggestions** | Clickable improved versions of the sentence in Search anything; choosing one replaces text without submitting. |
| **Autocomplete** | An optional next-word prediction accepted with the configured Accept next word shortcut. |
| **Grammar feedback** | English mistakes with simple explanations and clickable corrections. |
| **Accept next word** | The configurable action that inserts the visible predicted word. |

- **Model prompts** — editable system instructions sent to the selected local model for each English assistance feature.

| **App search path** | An absolute directory scanned for immediate executable files (including valid executable symlinks on Linux), or an individual executable included by Refresh. |

| **AI** | The Action and Search bar settings for asking one question inside Orbit using a local Ollama model, OpenAI or Gemini. |
| **AI provider** | Ollama, OpenAI or Gemini, selected for the AI Prefix. |
| **Model ID** | The provider-specific model name used for an AI request. |
| **API key** | A cloud provider credential entered in a masked field and stored in the system credential store. |
| **Ask** | Send the current AI prompt as a fresh question. |
| **Answer** | Selectable, temporary plain-text output of the AI request. |
| **Copy** | Copy the AI answer to the clipboard. |

| **Finished ID** | A saved reference to a completed item, including its source table and item ID. |
| **Achievements** | Today's completed items on the Home Screen; the full dated history is available in Data. |
| **Learn** | Mark the displayed Vocabulary entry as learned. Undo restores it to future selections. |
| **Next** | Select another saved local entry, without contacting a service. |
| **Quran Player** | A local passage widget with an action to open its saved recitation. |
| **Azan Reminder** | The saved prayer schedule and next prayer, with a date warning when stale. |
| **Habit Tracker** | Locally saved daily habits with completion checkboxes. |

| **Morning Remembrances (Adhkar)** | Offline Arabic morning remembrances with required repetition counts, source details and daily widget progress. |
| **أذكار الصباح** | Arabic heading for Morning Remembrances. |
| **Evening Remembrances (Adhkar)** | Offline Arabic evening remembrances with required repetition counts, source details and independent daily widget progress. |
| **أذكار المساء** | Arabic heading for Evening Remembrances. |
| **الفضل والمصدر** | Expand the supplied virtue and source details. |
| **تخطي** | Skip the current remembrance without completing it. |
| **السابق** | Return to the previous remembrance while retaining its count. |
| **إعادة الأذكار** | Restart this widget's daily remembrances. |

| **Azan settings** | Per-widget city, country and named time zone, with explicit Fetch times and a schedule preview accepted by Save. |
| **City**, **Country**, **Time zone** | Location fields used to request and interpret an Azan schedule. |
| **Fetch times** | Request today's prayer times for review without saving them until Save. |
