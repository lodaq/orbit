# Orbit design system

The launcher reference remains authoritative for pages and the dock. The Slint atlas and three PNGs in `add-item/` define sheets and controls. The atlas is a static gallery, not application code. Its colors are implemented in `ui/theme.slint`; shared controls live in `ui/controls.slint`.

## Foundations

Use embedded Inter. Titles are 20 px semibold, body and inputs 14 px, buttons and labels 13 px, hints and status 12 px. Primary text is #EFF0FF, secondary #9EA8C8. Sheets are #181D35, fields #14192E, hover #222942, borders #39415F, selected controls #303962, primary actions #6975E8 and focus #B6BEFF. Errors use #FFB0B8 on #392633. Backdrops use #060814B8. Never combine white panels with dark controls.

Use 4, 8, 12, 16, 24, 40 and 48 px spacing. Labels sit 6 px above their fields. Sheets have 16 px corners and inset; controls have 8 px corners. Borders are 1 px; keyboard focus rings are 2 px. Buttons and inputs are at least 40 px high; result rows are 52 px; previews are 48 px.

## Sheets and controls

Center sheets, cap width at 560 px, retain 24 px desktop or 16 px narrow-screen outer space. Keep the header and footer visible and scroll only the body. Content flows from the top at its natural height; hidden fields consume no space. Edit sheets fit their fields instead of reserving an Installed apps list. Text may wrap; button labels must not overlap trailing text or icons.

Use the same button for tabs, rows, sheet actions, Context menu actions and alerts. Selected, hovered, disabled and keyboard-focused states are distinct. Save and Choose use the primary action; Delete uses the error colors and always requires an alert. Cancel receives focus in a Delete alert. Close uses a single visible × with an accessible label.

Installed apps rows show their cached system icons. Missing system artwork uses the name; item tiles retain the Letter fallback. Resolve system icons only during Refresh installed apps; decode on the worker and retain the result. Editing an installed app exposes Name and Icon, never an empty Command field. Only custom commands have Command.

## Interaction

Open folders use the sheet colors, border and 16 px corners and padding. Their 20 px title sits above a scrolling grid with 16 px gaps. Each app has a 64 px icon, an 8 px gap before its 20 px label area, and an inset hover surface. The panel fits its rows up to the available screen height.

Add item opens from the Add tile. Selecting an Action or Installed app adds immediately; forms save explicitly. Refresh is manual. Filtering uses cached entries. Errors stay with the relevant field or operation and preserve drafts. Changing an icon uses a draft; Cancel discards it. Backdrop and Esc close the top layer. Showing and hiding Orbit remain instant.

Use the keys in shortcuts.md. The atlas's extra key proposals do not change Orbit's shortcuts. Do not implement the deferred Glyph picker as part of visual work.

## Review

Compare the real launcher against the supplied gallery: readable text on every panel, natural field spacing, full-width inputs, unclipped icons, visible Cancel and Delete actions, and no hidden controls intercepting input. Verify by opening Add from its tile and Edit/Delete from a real item's Context menu, including cancelling without changing saved data.

The Icon picker uses Letter, SVG, disabled URL and Choose icon tabs. Letter accepts one or two visible characters including emojis; SVG uses a labeled field; Choose icon opens the native file chooser. Save applies the draft, Cancel discards it. Custom command is the first and default App source. File and Directory sheets show Path, then Choose file or Choose directory, then a vertically centered Icon preview and Change icon control. Save is at the bottom right. Edit app also centers its Icon preview with Change icon.
