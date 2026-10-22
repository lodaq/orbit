# Orbit design system

The launcher reference remains authoritative for pages and the dock. The current `add-item/` HTML, PNG and PDF define Add item, with the retained behavior documented below; `doc/add-item/` supplies the earlier sheet atlas. These are references, not application code. `ui/theme.slint` is the source of truth for colors, spacing, typography and common geometry; shared controls live in `ui/controls.slint`.

## Tokens and components

Use `Theme` tokens for repeated colors, spacing, type sizes, control sizes, radii and borders. The spacing scale is 4, 8, 12, 16, 24, 40 and 48 px. Type roles are hint (12 px), control (13 px), body (14 px) and title (20 px). Common sizes include 40 px controls, 52 px result rows, 88 px multiline fields, 48 px previews, 8 px control corners and 16 px sheet corners. Add a token when a visual value is repeated or defines a shared role; keep one-off layout geometry local when it depends on content or available space.

Build repeated buttons and labeled fields from the shared controls. When a visual or interaction pattern is needed a second time, add it to `ui/controls.slint` and use it consistently. Keep each control's selected, hovered, disabled, error and keyboard-focus states distinct. New and updated UI follows these rules; migrate existing UI as it is touched rather than restyling the whole launcher at once.

## Foundations

Use embedded Inter. Titles are 20 px semibold, body and inputs 14 px, buttons and labels 13 px, hints and status 12 px. Primary text is #EFF0FF, secondary #9EA8C8. Sheets are #181D35, fields #14192E, hover #222942, borders #39415F, selected controls #303962, primary actions #6975E8 and focus #B6BEFF. Errors use #FFB0B8 on #392633. Backdrops use #060814B8. Never combine white panels with dark controls.

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
