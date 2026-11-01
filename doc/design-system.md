# Orbit design system

The launcher reference remains authoritative for pages and the dock. The current `add-item/` HTML, PNG and PDF define Add item, with the retained behavior documented below; `doc/add-item/` supplies the earlier sheet atlas. These are references, not application code. `ui/theme.slint` is the source of truth for colors, spacing, typography and common geometry; shared controls live in `ui/controls.slint`.

New and updated UI follows the relevant Apple Human Interface Guidelines (HIG), adapted to Orbit's Slint desktop launcher and existing behavior. Use legible semantic color, clear hierarchy, keyboard and screen-reader access, and motion that does not block input. Translucent surfaces over changing wallpaper need enough opaque tint and edge definition to keep their contents legible; use no blur unless Orbit's performance and visual specifications are revised.

## Tokens and components

Use `Theme` tokens for repeated colors, spacing, type sizes, control sizes, radii and borders. The spacing scale is 4, 8, 12, 16, 24, 40 and 48 px. Type roles are hint (12 px), control (13 px), body (14 px) and title (20 px). Common sizes include 40 px controls, 52 px result rows, 88 px multiline fields, 48 px previews, 10 px control corners and 20 px sheet corners. Add a token when a visual value is repeated or defines a shared role; keep one-off layout geometry local when it depends on content or available space.

Build repeated buttons and labeled fields from the shared controls. When a visual or interaction pattern is needed a second time, add it to `ui/controls.slint` and use it consistently. Keep each control's selected, hovered, disabled, error and keyboard-focus states distinct. New and updated UI follows these rules; migrate existing UI as it is touched rather than restyling the whole launcher at once.

## Foundations

Use embedded Inter. Titles are 20 px semibold, body and inputs 14 px, buttons and labels 13 px, hints and status 12 px. Primary text is #EFF0FF, secondary #9EA8C8. Sheets are #181D35, fields #14192E, hover #222942, borders #39415F, selected controls #303962, primary actions #5966D2 and focus #B6BEFF. Errors use #FFB0B8 on #392633. Backdrops use #060814B8. Never combine white panels with dark controls.

Labels sit 6 px above their fields. Borders are 1 px; keyboard focus rings are 2 px. Buttons and inputs are at least 40 px high.

## Sheets and controls

Center sheets, cap width at 560 px, retain 24 px desktop or 16 px narrow-screen outer space. Keep the header and footer visible and scroll only the body. Content flows from the top at its natural height; hidden fields consume no space. Edit sheets fit their fields instead of reserving an Installed apps list. Text may wrap; button labels must not overlap trailing text or icons.

Use the same button for tabs, rows, sheet actions, Context menu actions and alerts. Selected, hovered, disabled and keyboard-focused states are distinct. Save and Choose use the primary action; Delete uses the error colors and always requires an alert. Cancel receives focus in a Delete alert. Close uses a single visible × with an accessible label.

Installed apps rows show their cached system icons. Missing system artwork uses the name; item tiles retain the Letter fallback. Resolve system icons only during Refresh installed apps; decode on the worker and retain the result. Editing an installed app exposes Name and Icon, never an empty Command field. Only custom commands have Command.

## Interaction

Open folders use the sheet colors, border and 16 px corners and padding. Their 20 px title sits above a scrolling grid with 16 px gaps. Each app has a 64 px icon, an 8 px gap before its 20 px label area, and an inset hover surface. The panel fits its rows up to the available screen height.

Add item opens from the empty-space Context menu. Pages have no Add tile. The menu stays within the window, closes with Escape or an outside click, and clears on page changes or hiding Orbit. Selecting an Action or Installed app adds immediately; forms save explicitly. Refresh is manual. Filtering uses cached entries. Errors stay with the relevant field or operation and preserve drafts. Changing an icon uses a draft; Cancel discards it. Backdrop and Esc close the top layer. Showing and hiding Orbit remain instant.

Use the keys in shortcuts.md. The atlas's extra key proposals do not change Orbit's shortcuts. Do not implement the deferred Glyph picker as part of visual work.

## Review

Compare the real launcher against the supplied gallery: readable text on every panel, natural field spacing, full-width inputs, unclipped icons, visible Cancel and Delete actions, and no hidden controls intercepting input. Verify by opening Add item from the empty-space Context menu and Edit/Delete from a real item's Context menu, including cancelling without changing saved data.

The Icon picker uses Letter, SVG, disabled URL and Choose icon tabs. Letter accepts one or two visible characters including emojis; SVG uses a labeled field; Choose icon opens the native file chooser. Save applies the draft, Cancel discards it. Custom command is the first and default App source. File and Directory sheets show Path, then Choose file or Choose directory, then a vertically centered Icon preview and Change icon control. Save is at the bottom right. Edit app also centers its Icon preview with Change icon.

Toasts use compact raised cards with a small status icon, stacked at the bottom right with 24 px outer space and 8 px gaps. Confirmations and information dismiss after 3 seconds, warnings after 5 seconds and errors after 7 seconds, including while hovered. Clicking a Toast dismisses it immediately.

## Search cursor

Search anything retains the native TextInput for editing, wrapping, selection and IME. Smooth cursor overlays its final caret coordinates with a Theme.cursor-color blue block at 0.65 opacity. Theme cursor tokens define a 0.55 font-size width, 70 ms lead, 180 ms trail, 160 px maximum additional stretch, 450 ms idle delay, 1100 ms fade leg, 0.1 idle minimum and 80 ms selection fade. Line changes and window show snap. Edits reset idle even without position changes. The overlay is removed on hide, unfocus, disable or IME composition, stopping its timers and animations; native IME and disabled carets retain the style width. Selection fades only the overlay.

Slint 1.18.1 exposes cursor/anchor byte offsets and preedit-text as undocumented properties. The event-driven Rust bridge in src/search-cursor.rs disables native caret visibility while the overlay exists and restores the native blink binding when disabled or composing IME text. It uses the pinned Slint version’s internal safe API. Recheck these APIs on upgrades.

Native caret suppression uses cursor-visible, not width: both 0 px and a tiny positive width can still draw a blinking GPU hairline. The bridge runs only for input/focus/state changes; animation remains in Slint. Regression checks cover multiple native blink cycles.

## Shortcut keycaps and search layout

`Kbd` in ui/controls.slint is Orbit’s native Slint equivalent of the rounded, muted shadcn/ui Kbd visual (https://ui.shadcn.com/docs/components/base/kbd). Use it for displayed shortcut chords and keys throughout the app: search addon, footer hints, Settings close hint, saved bindings, recorder preview and Shift + wheel help. A chord can occupy one keycap. Escape and direction keys display as Esc and arrows; accessible labels retain the complete saved name. Keycaps have 22 px height, 6 px side padding, 5 px corners, 11 px type, subtle border and bottom edge; all colors and dimensions come from Theme. Long chords elide within their available space. They are labels, so mouse and keyboard events still reach the containing button or input.

The search bar keeps its approved position and 560 px maximum width, with a 48 px minimum height, 16 px input text, consistent 12 px outer padding and a quiet focus border. The Prefix badge aligns with the first text row. The keycap addon uses the first saved Focus Search anything binding; footer and close hints likewise follow the saved shortcuts, including removal.

Opening a folder moves its panel from the source tile to the center over 150 ms while scaling it from 0% to 100%; panel width and height do not animate. The backdrop and content fade in. Closing reverses the movement. The timing comes from `Theme.folder-transition`.

The wrapped text width reserves the entire blue block width plus 2 px inset on each side. Text measurement uses exactly the input width. Cursor coordinates include the input inset and Flickable offset. Vertical scroll decisions use the actual full cursor height and retain 2 px bottom clearance, including the final line at the maximum search height. The overlay stays clipped to its viewport while the whole resting block remains visible.


## Settings

`settings-*` tokens in `ui/theme.slint` and shared Settings controls in `ui/controls.slint` implement `settings/DESIGN.md` and its handoff palette. These tokens apply only to Settings; the top bar, dock, footer hints and other sheets retain their existing appearance. The sidebar is 232 px wide, the content scrolls below its fixed title and Search bar segmented control, and Back stays at the bottom of the sidebar. Prefix and Action grids use three columns at 1100 px content width, two at 640 px and one below; General and Shortcuts split at 900 px.

Shortcut chips have a removal × followed by confirmation; Add records a replacement. Chip labels are display-only. Exclusion chips remove a file-search pattern immediately and persist the list. All controls retain visible keyboard focus and accessible labels. Status and inventory text use real state, never mockup values.

Settings chip removal buttons are vertically centered. Settings steppers have spaced, inset minus and plus buttons; Item size uses the same stepper with an 80–176 px range and 8 px increments. Widgets keep their original size while being dragged; app and folder lift feedback remains unchanged.

## Add item redesign

`add-item/Main.dc.html`, `Add item@1x.png` and `Add item dialog redesign.pdf` define the Add item visual reference. Implement it natively with `add-*` Theme tokens, AddOption/AddField's opt-in `add-style`, and AddItemRow. These styles do not change Edit sheets, the Icon picker or Settings. The exported JavaScript and its sample inventory are reference material only.

Add item retains a 560 px maximum width, content-fitting height capped at 760 px, and a scrolling body between its fixed header and separated footer. Its six icon type tiles wrap on narrow windows. App source tabs use an underline; Custom command remains first and selected by default and retains Change icon. Actions are grouped into System, Audio and Media with explicit Add buttons; Installed apps uses cached system icons, real command descriptions and explicit Add buttons. Refresh remains manual. All successful additions close the sheet and duplicates remain allowed. Incomplete or locally invalid forms disable Save; worker validation failures retain the draft. File and Directory place the native chooser alongside Path when space permits, otherwise below it. Widget previews retain the actual Tasks 2×2 and Focus Task 4×2 dimensions at the current Item size.

Add item anchors its top edge at the position of a centered maximum-height sheet (760 px, or the viewport height minus 32 px). Changing type or App source animates only the height over 180 ms; the header remains stationary and the body scrolls beneath it. Website and Edit website offer Automatic (the default for fetched icons), Custom icon, a preview and retry feedback. Automatic fetches after a 400 ms URL typing pause on a background worker. Save waits for that preview, then reuses its bytes rather than fetching again; failed fetches allow a letter icon. Preview bytes stay in memory until Save. Picker Cancel preserves the current choice, and Automatic restores fetching. Existing fetched files and chosen-* custom images retain their mode through folder conversion without changing the config format.

## Apple design pass

The app-wide design pass retains Orbit's artwork, embedded Inter, dark palette and Linux/Windows shortcuts. Never display Option or Command in place of Alt or Ctrl. Add item and Settings share the main text, focus, border and primary-action colors; their structural surface shades still distinguish each region.

Shared buttons respond immediately on pointer-down with a light surface overlay and ease back over 120 ms after release. The hit target never moves. All shared buttons expose their default accessibility action and take keyboard focus when clicked. Selected and destructive states remain visible beneath press feedback. Settings switches move their thumb over 120 ms with no overshoot; selectors distinguish pressed, hovered, focused and disabled states. Card titles use semibold weight. Sheets and alerts use the shared 20 px corners and a soft downward shadow to establish depth without adding blur to the dock or startup work.

Showing and hiding remain immediate. Existing folder origins, page tracking and widget interactions retain their established behavior. No sound, haptics or macOS keyboard conventions are added by this design pass.

## Complete interface design contract

This pass applies the Apple design and Frontend UI Engineering skills to every existing surface, using native Slint. The referenced exported design directories are absent from this checkout; the running launcher, Add item and Settings establish the reference hierarchy. Orbit keeps its original artwork, Inter, lavender accent, instant show/hide and desktop keyboard conventions.

- Launcher, search and dock: keep launchable content dominant, use immediate press feedback without moving hit targets, retain visible keyboard focus, and use a quiet translucent dock without blur. Search results share the tile language and keep location text secondary.
- Widgets: one material, border, header and text hierarchy for local and TickTick widgets. Content remains scrollable; completion and refresh retain their existing behavior. Habit controls include the habit's actual name for accessibility.
- Settings: a darker structural sidebar, calmer grouped content, consistent section titles, fields, selectors and action placement. Compact desktop widths reduce sidebar and gutters; text remains readable rather than shrinking.
- Context menus and selectors: anchored floating surfaces with flat, left-aligned rows, a visible selection indicator, explicit destructive labels and a subtle shadow. Opening a nonmodal menu does not dim the whole launcher.
- Add/Edit, task editing, Icon picker, Move to Page and alerts: one panel material, 20 px panel corners, 24 px content insets where space permits, visible Cancel and primary actions, and a dimmed backdrop. Headers and small controls use the same type hierarchy. Narrow panels fit inside the viewport.
- Folders: preserve their source-anchored movement, give contents the same tile feedback, and keep a clear heading above the scrolling grid.
- AI, English assistance, Data and Toasts: use the same surface and type tokens, preserve selectable answers and real loading/error/empty states, and never add a decorative loading delay.

Use 10 px control corners, 22 px widget corners and 20 px sheet corners. Motion is short and non-bouncing for discrete controls; dragging remains direct. No full-screen background motion, stacked translucent text layers, macOS key names, new network work, or replacement artwork.

Primary actions use a slightly deeper lavender so white control text clears 4.5:1 contrast. The original icon and background artwork is retained. Settings uses labeled switches and shared multiline fields for English assistance; task lists, priorities and Move to Page use the shared selector. At narrow desktop widths, Settings reduces its sidebar and gutters. Page-edge drags resist progressively and return without a hard stop.

The TickTick header click region includes its visible New task (+) control; press and release there opens the task editor.

Memory follow-up (2026-10-25): runtime checks during this pass measured approximately 151–160 MB RSS, above the existing 140 MB hidden-idle budget. The user chose to finish the UI pass and investigate memory separately; the budget is unchanged. Windows runtime, screen-reader output and system reduced-motion preferences have not been verified in this Linux session.

The task editor keeps its header and actions outside a scrollable, top-aligned field body. New task uses a compact 512 px maximum height; task details use 730 px, both bounded by the viewport.

UI verification (2026-10-25): native Hyprland paths exercised the populated launcher, dock, search, AI view, folders, empty-space and item menus, Add item → Icon picker, Delete → Cancel, Move to Page, General, Search bar, Prefixes, Actions, English assistance, AI settings and Data. Compact Settings was checked at a 1024×576 logical desktop using Slint scaling. A large saved widget layout did not fit that smaller page, so compact page reflow is not accepted by this UI pass. Screenshots from the isolated session are in `target/ui-check/apple-design/`. No TickTick task was created, changed or deleted during the checks.


## Widget grid and dock alignment

Add item → Widgets uses a two-column gallery at 440 px content width and above, and one column below. Cards use shared field/widget surfaces, text roles, borders, radii and buttons. Each card keeps its name, local-data or footprint description, proportional preview, Auto and Place together. The sheet body scrolls; the header and Cancel stay visible. Auto and Place preserve their existing placement behavior.

The dock fits its actual item slots with equal inner padding, including a single item. Its empty state remains visible. Footer shortcut hints are removed; configured shortcuts, Search anything's shortcut hint and Settings shortcut controls remain available.

All native UI files use Theme's repeated type sizes, surface radii, placement feedback colors and common surface colors. Application and search-engine artwork retain their original brand colors.

Verification: rebuilt release opened through empty space → Add item → Widgets; Auto added Random Quote, Place added Random Fact at the selected position, and scrolling reached Habit Tracker. Single-item and populated docks showed balanced padding, and both footer hints were absent. The normal profile was restored on the current release. Captures: `target/ui-check/widget-grid/`.


## Data collections

Settings → Data uses the supplied October 28 reference: a heading and actual local storage meter above grouped collection navigation, a bordered entry list, and an adjacent editor. Collection counts reflect all saved rows, independently of search. Attribution sits beneath content, and the edited row has a selected surface plus a leading accent. The editor uses shared Settings controls, a taller scrollable Content field, Unicode character counts, inline errors and bottom-aligned Cancel/Save. The reference's error is a state example and appears only when validation fails.

The list keeps search and Add entry above its scrolling rows, and entry count and pagination below. All existing category-specific fields, learned-word undo, confirmed deletion and read-only Achievements remain available. Below 1050 px of content width the editor stacks below the list; the Settings body scrolls to it. No entrance or list animation delays reading or editing.

Verification (2026-10-28): the rebuilt release was opened through Launcher → Settings → Data in an isolated X11 display and profile. Confirmed collection counts and switching, selected-row editing, character counts, validation, Save persistence, Add entry, Enter-to-search, confirmed deletion, read-only Achievements and the stacked editor with reachable Save at 1280×800. Formatting, Clippy and 106 logic tests passed (six opt-in tests ignored). Captures: `target/ui-check/data-redesign/`. The normal hidden daemon was restarted on the final release.

## Widget reading hierarchy

Widget redesign keeps Orbit's lavender palette, embedded Inter, existing placement and local data. Quiet 13 px headers and the shared widget surface unify the eleven widgets. Content establishes their differences: a restrained quotation mark for quotes, 20 px reading type for quote/fact/motivation/affirmation/Quran content, a 24 px vocabulary headline with 14 px definitions, a separate next-prayer headline and schedule, grouped habit rows, and a compact achievement list. Details use the shared secondary text role. Gallery previews reflect each widget's content.

Local controls share a 32 px action height and a footer separator; scrollable content cannot push actions outside the widget. The header and resize corner remain available to the page's layout gestures. Resize hints become prominent on hover. TickTick keeps its existing list, add, menu, task-row and focus-task hit regions, using quiet headers and inset header actions. Its completion cue changes opacity instead of width/height. Launcher show/hide and content changes stay immediate.

| Before | After | Why |
| --- | --- | --- |
| Similar small type for every local widget | Content-specific type hierarchy | Distinguishes reading, learning and daily progress |
| Full vocabulary entry enlarged together | Word headline with smaller supporting text | Makes the word easy to find without crowding the example |
| Plain habit checkbox list | Grouped habit rows with a completed surface state | Makes daily progress easier to scan |
| Tinted TickTick header strips | Quiet shared surface with inset actions | Keeps attention on tasks while preserving controls |
| Expanding completion overlay | Brief opacity feedback | Avoids animating layout for a frequent action |

Widget verification (2026-10-25): release opened in an isolated two-page layout containing all eleven widget types. Checked bilingual quote/Quran content, vocabulary headline and definition, next-prayer hierarchy, habit completion/undo, Affirmation Done/Undo, Vocabulary Learn/Undo, Refresh, and TickTick + → New task → Cancel. The real empty-space → Add item → Widgets → Auto path added Vocabulary Builder at its default footprint; scrolled its content, activated Next with the keyboard, moved it by its header and resized it by the corner. Captures are in `target/ui-check/widget-redesign/`. No TickTick task was saved or completed.


## Widget material and resize grip

Widgets use a subtle lavender material (`widget-background`), a fine light border and 22 px corners. This applies to local and TickTick cards. Habit Tracker uses full-row custom circular completion controls with keyboard focus and checkbox accessibility, a Today heading, a completed/total count and a thin progress track. Completion remains immediate; subdued completed text and material convey state without replacing the content. The previous native square checkboxes and standalone date label are removed.

The triangle resize indicator is replaced by an inset curved corner grip with a 28 px visual area inside the existing 32 px resize hit region. It highlights on hover and during resizing, appears above TickTick content as well as local widgets, and hides when layout is locked. It adds no pointer interception or movement animation.

| Before | After | Why |
| --- | --- | --- |
| Square platform checkbox inside a tinted block | Circular completion control and a clickable row | Gives habits a coherent, generous target |
| Raw date under habits | Today and actual completion progress | Makes daily state clear at a glance |
| Flat widget frame | Soft material, finer border and rounded corners | Separates widgets without heavy chrome |
| Bright triangle in the corner | Rounded inset corner grip | Makes resizing discoverable without a sharp decorative glyph |

Material/grip verification (2026-10-25): rebuilt launcher opened an isolated profile through page navigation. Clicked habit labels to complete and undo, confirmed the count/progress changes, resized Habit Tracker from the new corner grip (360×288 to 424×320 saved bounds), and confirmed layout lock hides all local and TickTick grips while habit actions remain available. Reviewed the other local card materials on the content page. Release build, formatting and Clippy passed. Native preview: `target/ui-check/widget-material/habit.png`.

## Search locations

Search bar → General uses one shared Settings card for app, file and directory paths plus file exclusions. A header-level Refresh action rescans all sources. Quiet separators group the controls; file and directory paths share a two-column layout that stacks below 640 px. Each source explains its scan depth. Refresh is also available from General and Installed apps and always rebuilds both caches on the worker.

Verification (2026-10-27): rebuilt release opened from Settings → Search bar → General; reviewed the combined header and scrolled to file/directory paths and exclusions. An isolated X11 display running the release launcher confirmed a file created after the first search remains absent until Refresh, then appears without restart. Captures: `/tmp/orbit-isolated/`. Formatting and Clippy passed.

## Search result placement

Search result Context menus use the shared floating Panel and MenuOption controls for Pin and Add to Page. Dragging retains the source pointer target while hiding the result surface and English assistance, revealing the existing Page and placement preview. Free-space placement, dock pinning and page-edge navigation use the existing spatial rules. Release commits once; Esc or an invalid drop preserves the search. Search engine fallback tiles remain launch-only.

Verification (2026-10-27): the release launcher on an isolated X11 display passed Context menu → Add to Page, Context menu → Pin, result drag → Page, result drag → dock, Esc cancellation, moving an existing saved result without duplicating its ID, and page-edge creation followed by placement. Checked saved paths, types, positions, exclusive dock ownership and unchanged config timestamps for repeated Pin or Add to Page. Captures: `target/ui-check/search-results/`.

## Azan reading card

Azan Reminder follows Apple Design's hierarchy, familiarity and immediate feedback while retaining Orbit's material and Inter. A settings icon aligns with the widget title at the top right; a quiet location row sits below it; the next prayer and 34 px time form the visual focus. Daily rows align names and times and subtly highlight the upcoming prayer. A centered settings sheet keeps its header/actions visible, scrolls its fields and fetched preview, and dismisses through Cancel, backdrop or the configured Close shortcut. New motion and blur are unnecessary for this reading task.

TickTick picker and rows (2026-10-28): use a bounded scrollable Lists & tags panel with section labels and a selected-row checkmark. Header actions share 32 px surfaces and drawn plus/ellipsis icons. Priority checkbox colors come from `priority-high`, `priority-medium` and `priority-low`; Arabic-first rows mirror checkbox/date placement and clip title glyphs inside their text area.

Verification: the release launcher on an isolated X11 display exercised header → Lists & tags → scroll → tag selection, persisted the selected filter, opened New task → Cancel and the options menu, and confirmed Arabic title alignment, checkbox placement, focus-task title placement and immediate accumulation of rapid wheel input. No TickTick task was changed. Captures: `target/ui-check/ticktick-redesign/`. Release build, formatting and Clippy passed; all 113 enabled tests passed (six existing opt-in tests ignored).

Focus task (2026-10-28): Apple-style hierarchy puts a 24 px Inter title (30 px Amiri for Arabic) above a quiet priority capsule, date, tags and notes. The completion ring responds on press; the header remains a drag surface. Open task and Details form a quiet fixed footer. The compact layout retains a 32 px completion ring and scrollable body. Rendering and scroll limits share the FocusTaskBody component, keeping long wrapped titles, notes and expanded metadata reachable.

Focus verification: the release launcher on an isolated X11 desktop exercised Focus task → Details → resize to a 160 px card → scroll to the final metadata line → resize back → Hide details, and Open task → Task details → Cancel. A second focus card confirmed independent expansion and scrolling. The content-formatting test preserves notes/checklist text without technical metadata in the main view. No TickTick task was modified. Captures: `target/ui-check/focus-redesign/`. Release build, formatting and Clippy passed; all 114 enabled tests passed (six existing opt-in tests ignored).

## Achievements reading card

Achievements keeps the shared widget material and draggable header. Its body starts with Today and the saved date, followed by a prominent count of completed items. A quiet separator leads into individually spaced, wrapping rows with read-only circular checkmarks, aligned at the top of the list area even when there are only a few. The list scrolls independently; the summary remains visible. Small cards reduce the count size and spacing. An empty day shows zero and the existing empty-state text. No progress target, action button or entrance animation is added.

Verification (2026-10-28): the release launcher on isolated X11 display :96 followed empty space → Add item → Widgets → Achievements → Auto. Checked live habit completion/undo, empty Today state, eleven completed items, long mixed English/Arabic wrapping, scrolling to the last rows, and corner resizing from 272×272 to 377×394. Formatting, Clippy and all 106 enabled logic tests passed (six opt-in tests ignored). Captures: `target/ui-check/achievements-redesign/`.

TickTick achievement labels (2026-10-28): completion callbacks and synchronization carry the task title alongside its Finished ID. Schema 6 retains only the title needed for the label, including history and offline restarts; empty updates preserve an existing title. Verified English and Arabic names through Launcher → Achievements and Settings → Data → Achievements on isolated display :95. Regression coverage checks migration, restart, history, updated/blank names and undo. All 107 enabled tests, formatting, Clippy and the release build passed. The normal hidden daemon was restarted; its existing refresh backfilled all of today's TickTick references. Captures: `target/ui-check/achievement-task-titles/`.

Arabic text uses the shared `OrbitText` control and content-aware field alignment; the permanent text-direction rule is in `rules.md` → Arabic text. Vocabulary body lines align independently so Arabic examples remain right-aligned beside English definitions.
