# V2 authority

The user approved `Orbit_Desktop_Launcher_UX_Spec_v2.md` on 2026-10-20 as authoritative wherever it conflicts with this document. Windows and Arch Linux, spatial grids, widgets, exclusive dock ownership, new UI terms, and the complete v2 build sequence are authorized. Existing visual artwork and nonconflicting performance and coding rules remain in force. The older build-step restriction does not apply to v2.

# Orbit shortcuts

With Canvas focus (click a tool or empty Canvas space), Ctrl+Z undoes the last content edit and Ctrl+Shift+Z redoes it. Ctrl+X cuts selected items, Ctrl+C copies them, and Ctrl+V pastes them near the visible center without overlaps. History holds 20 edits for the current session; pan/zoom are not history steps. These edit commands also appear in the Canvas Context menu. Lock layout allows Copy and blocks the other edits. Native text editing keeps these keys in text fields.

Ctrl+J and Ctrl+L fit the next Group; Ctrl+H and Ctrl+K fit the previous Group, following saved order and stopping at the ends. Outside Canvas focus, Ctrl+K keeps its Search anything action. Keys 1, 2, 3 and 4 select Select, Hand, Draw and Eraser. Canvas edit, Group navigation and tool bindings can be changed in Settings → Shortcuts. Existing conflicting assignments are preserved during migration.

Wheel pans vertically, Shift+wheel pans horizontally and Ctrl+wheel zooms around the pointer. Draw shows ink and Eraser removes strokes during the drag; release saves once and Escape cancels the gesture.

Canvas tools: Select drags a selection rectangle; Ctrl-click or Shift-click toggles items, and Ctrl/Shift-drag adds a rectangle to the selection. Hand drags to pan. Draw saves ink; Eraser removes whole strokes. Right-click with a selection → New Group to name it and record a shortcut. Click a Group label or press its shortcut to fit its items. Right-click the label to edit or delete the Group. Escape cancels an active gesture, then clears selection. Group shortcuts require Ctrl, Alt or Super and run only on the Canvas with no layer open.

Orbit is mouse first. Settings → Shortcuts lists Orbit bindings and lets you add or delete them. To change a binding, use its ×, confirm with ✓, then use Add to record a replacement. Chip labels do not start recording. Choose an action and record a key combination; duplicate combinations are rejected. The tables below are defaults. Text editing shortcuts are provided by text fields, and the global toggle remains in Hyprland. Ctrl+K returns focus to the search field; while results are visible, arrows navigate results and typing edits the query.

## Launcher

The launcher opens with Search anything focused. Ctrl+K focuses it again and Enter opens its selected result; when no local items match, search engines appear with the default first and selected; Enter searches using the selected engine. Esc clears search before hiding the launcher. Grid arrows move between spatial neighbors, Enter opens the focused item, Shift+F10 opens its Context menu, and no shortcut changes pages.

| Action | Shortcut |
|---|---|
| Move left in search results / Canvas focus | `←` |
| Move focus down | `↓` |
| Move focus up | `↑` |
| Move right in search results / Canvas focus | `→` |
| Open (launch an app, open a Website, File or Directory, open a Folder) | `Enter` |
| Close the top layer (picker, then sheet or alert, then open folder, then launcher; on Settings, go back) | `Esc` |
| Open Settings; press again to return to the launcher | `Ctrl+S` |

Open Settings toggles Settings with `Ctrl+S` by default and can also be assigned to another combination. Legacy Add page and Delete page bindings are removed without renumbering the remaining actions. Deleting the final binding disables an Orbit action’s shortcut; its mouse control remains available.

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

Type a Prefix Token followed by Tab or Space to activate it. The Prefix name appears as a badge and search results are hidden until the Prefix is cleared. Backspace removes the active Prefix when the query is empty. Enter sends the remaining text to its Action. For a TickTick task, type `t`, press Tab or Space, then enter `Buy milk !today !high #home ~Buy` and press Enter. `!today` or `!!` sets today's due date; `!high`, `!medium`, `!low` and `!none` set priority; `#tag` adds a tag; `~` and `^` select a list. Put a multiword list name at the end, for example `~hall of fame`. Leading emoji may be omitted when the readable name identifies one list. Omit the list to use Inbox. Task creation updates the widget immediately, then confirms the save or rolls back with a specific error.

Search engines: `:br` Brave, `:g` Google, `:d` DuckDuckGo, `:q` Qwant, `:b` Bing, `:sp` Startpage and `:e` Ecosia. Other site-search Tokens are `g` or `ل` for ChatGPT, `c` or `ؤ` for Claude, `gh` for GitHub, `m` for Google Maps, `i` or `ه` for Image Search, `p` or `ح` for Perplexity, `w` or `ص` for Wikipedia, `ف` for Arabic-to-English Translate, `tr` for English-to-Arabic Translate, and `y` or `غ` for YouTube. Type a Token, press Space or Tab, enter the query, then press Enter.

- `←`, `→`, `↑`, `↓`: move focus to the nearest item in that direction.
- `Enter`: open the focused item.
- `Shift+F10`: open its Context menu.

## Mouse (fixed)

| Mouse | Result |
|---|---|
| Click an item | Launch an app, open a Website, File or Directory, or open a Folder |
| Right-click empty Canvas space → Add item | Open the Add item sheet |
| Click a System action | Add it as an App |
| Click an installed app | Add it to the Canvas |
| Choose a file or directory in its Add item sheet | Fill Path; Save adds an item named after the path |
| Right-click a Canvas item | Context menu: Edit, Pin or Unpin, Delete |
| Right-click a pinned dock item | Unpin |
| Right-click a Recent dock item | Remove from Recent |
| Drag an item | Move it on the Canvas or into a Folder |
| Wheel up / down | Pan vertically; Shift pans horizontally; Ctrl zooms around the pointer; widget content scrolls normally |
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
5. **The dock is the last row.** `↓` from the bottom row of the Canvas enters the dock, `↑` goes back up, `←` and `→` walk across the separator.
6. **A drag in progress** is cancelled by `Esc`.
7. **The focus ring is for keyboard use.** It is hidden when the launcher opens and appears when you press a key. Using the mouse hides it again.
8. **Toggle Orbit is not a key in Orbit.** The toggle key lives in `hyprland.conf`.
9. **Any keyboard layout.** All keys here work the same on every layout.

Search anything: **Ctrl+W** deletes the previous word using native text editing, including selected text and undo. When the query is empty, it removes the active Prefix. Manage or remove **Delete previous word** in Settings → Shortcuts. It acts only while Search anything is focused with no Settings or layer open. Existing bindings are preserved; an already assigned Ctrl+W is not replaced.

The top-bar Lock layout / Unlock layout button has no default shortcut. While locked, layout changes are disabled; pan, zoom and Show all items remain available.

English assistance: **Accept next word** uses Ctrl+E by default while Search anything has a visible next-word prediction at the end of the query. It is configurable in Settings → Shortcuts. Existing Ctrl+E assignments are preserved; Tab also accepts a visible prediction; an exact Prefix Token takes priority.

Personal Hyprland binding: **Alt+T** runs `orbit ticktick`, opening Search anything with the TickTick Prefix active. Type a task and press Enter.

AI Prefix: type **ai**, then **Space** or **Tab**. Enter submits a fresh question to the selected Ollama, OpenAI or Gemini model and displays the answer inside Orbit. Ask submits with the mouse; Cancel invalidates the pending reply; Copy copies the answer. Backspace on an empty prompt or Esc clears the Prefix through the existing navigation rules.

While AI is active, hidden-result navigation does nothing. Arrow keys edit the prompt or select answer text normally; Enter in the prompt still submits the AI question.

The launcher footer does not display shortcut hints. The shortcuts themselves and Settings → Shortcuts remain available.

Search results: right-click → Pin or Add to Canvas. Drag a result to reveal the Canvas, then drop on free space or the dock. Esc cancels without adding the item. Lock layout disables placement.

Search bar shortcuts keep their English letter positions on Arabic layouts, including Ctrl+A (select all), Ctrl+E (accept the available next-word prediction), Ctrl+W (delete previous word), clipboard and undo shortcuts. Ordinary Arabic typing is unchanged. Arabic-leading search text aligns right; English-leading text aligns left.

Accept next word also completes the current partial word. The Autocomplete row shows the exact word or spelling replacement Ctrl+E and Accept word will apply; the inline ghost shows only the untyped suffix. A grammar issue is applied by clicking that issue, independently of Ctrl+E.

English Suggestions: Ctrl+1, Ctrl+2 and Ctrl+3 apply the corresponding displayed rewrite. Ctrl+N or Ctrl+J highlights the next suggestion; Ctrl+P or Ctrl+K highlights the previous one. Enter applies the highlighted suggestion without submitting. Navigation stops at the ends; the first down/up selects the first/last suggestion. Editing or refreshing suggestions clears the highlight. While suggestions are available in Search anything, Ctrl+K navigates them; otherwise it retains its search-focus action.

Canvas: Select drags a selection rectangle; Hand drags to pan; Ctrl+wheel zooms around the pointer from 25–200%. Right-click empty space → Add item opens the existing sheets and widget gallery at the clicked position. Right-click → Show all items fits the arranged content, including below 25% when necessary. Widgets scroll their own content. Escape cancels an active drag or placement preview without saving. Lock layout prevents adding, moving and resizing items while permitting navigation and widget interactions. The saved view survives hiding and restart. There are no page-switch actions or Box controls.

Robot workbench: Escape closes the workbench without cancelling work. Typing, Tab, text selection and clipboard shortcuts stay in the workbench while it is open. Send queues the chat message with the mouse; Cancel work cancels the selected conversation's queued/pending work. Approve once applies only to the displayed conversation and action. Delete conversation requires confirmation and is unavailable while that conversation has work outstanding.

Scripts: type `/` to list saved scripts, then filter by name. Up/Down select without wrapping. Tab fills the selected script name and preserves arguments. Enter completes a partial name first, then runs on the next press; a complete name or a query with arguments runs immediately. Esc clears script mode. The first space separates the name filter from one unchanged argument string. With no match, Enter does nothing. Rows can also be clicked.

Tab and Enter script-name completion append a space when no argument separator exists, placing the cursor ready for arguments. Existing argument text and spacing are preserved.
