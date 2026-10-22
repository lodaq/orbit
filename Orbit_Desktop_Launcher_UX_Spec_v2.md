# Orbit Desktop Launcher — Android-Style Grid, Folders, Pages, and Widgets

**Implementation-ready interaction and UX specification for Codex**
**Target platforms:** Windows and Arch Linux (desktop only)
**Reference:** Existing Orbit desktop screenshot; Android launcher interaction recording; useful mechanics from the attached Claude analysis.
**Scope:** User-facing behaviors, state rules, gestures, geometry, persistence, animations, and acceptance tests. Implement in the existing codebase; this is not a request for a new prototype.

## 0. Non-negotiable principles

1. **Keep Orbit's existing desktop UI**: top Orbit header/search, wide rounded workspace, page dots and Add Page control, and persistent bottom dock. Do not replace it with a portrait phone simulator, mobile status bar, mobile breakpoint, or fixed 3×4 layout.
2. **A spatial grid, not a sortable list**: every top-level app, shortcut, folder, or widget has an explicit page and grid position. Empty cells are valid and persistent. Movement into an empty cell does not compact or shift other items.
3. **Configurable sizing and spacing**: the user can change **item size and item gap in the existing application Settings page**. The spatial grid uses fixed 1 px cells; each item spans enough cells to preserve its physical size. Calculate grid capacity from available *desktop workspace* dimensions; do not use separate mobile/tablet layouts.
4. **Center = folder; edges = reorder**: when dragging an **app over another app**, the central zone of the target app is reserved for potential folder creation; the outer edge/insertion zones are reserved for reordering. These targets MUST have different hover states, timers, previews, and outcomes. No accidental folders.
5. **Folders are first-class grid items**: drag app onto app to create; drag app onto folder to add; open, rename, reorder children, extract children, move folder, dissolve on one remaining child if enabled.
6. **Widgets are first-class grid items**, with multi-cell footprints, configurable size in cells, and a built-in widget catalog. Widgets may use the internet and connect to external services when the user configures them. Keep authentication and stored credentials secure, run network work off the UI thread, and show recoverable errors when a service is unavailable. TickTick Tasks is the only supported widget, as requested on 2026-10-20.
7. **Pages are real separate grids**: smooth horizontal sliding, clickable page indicators, add/remove/reorder pages, edge-hover cross-page dragging, persistent shared dock.
8. **The UI always previews the exact result before committing it**; preview must never mutate saved layout. A drop succeeds atomically or the item animates home.
9. **Do not uninstall or delete underlying apps/files** when the user removes a launcher shortcut. Use 'Remove from Home' as the primary action. Treat actual OS uninstall as out of scope.

## 1. Desktop surface and sizing

### 1.1 Layout

Retain the Orbit layout: header above; wide main home-screen workspace in the center; indicators beneath it; persistent dock below. The grid starts at the top-left of the workspace with deliberate padding and continues through the usable interior. The existing Orbit cards/visual theme remain recognizable.

There is **no phone portrait canvas, Android status bar, mobile navbar, touch-only gesture, or phone-specific dimensions**. Support resizable windows and desktop monitors on Windows and Arch Linux, including different resolutions and display scaling. The underlying interaction model must not vary by OS.

### 1.2 Settings: item size and gap

Add a setting to Orbit's **existing Settings page**:

- Label: `Home Screen > Item Size`.
- Label: `Item Gap (px)`; integer range 0–64, default 16.
- Show slider and numeric value, with a live preview; offer `Reset to default`.
- Initial suggested range: **80–176 logical CSS pixels**, step **8 px**, default approximately **112 px**; adjust default based on the existing Orbit card dimensions. Do not hardcode these numbers throughout the app.
- Item Size is the source of truth for app and folder card size. The spatial grid remains 1 px per cell; item spans scale with the setting so cards retain their chosen physical size. Item Gap controls the inset on each occupied span, preserving 16 px by default.
- Widgets use spans derived from their design dimensions and the current item size (e.g. 2×1 or 2×2 cards), so their physical size follows the setting.
- Grid capacity derives from available workspace width and height at one cell per logical pixel. Item size and spacing remain separate.
- A setting change must update both rendering and hit-testing/drag geometry. The preview and final drop cell may never disagree.

Example calculations (illustrative, not rigid constants):

```ts
columns = Math.max(1, Math.floor(workspaceWidth - 2*padding));
rows = Math.max(1, Math.floor(workspaceHeight - 2*padding));
```

When resizing Orbit, the **number of visible columns/rows can change**; these are desktop adjustments, not mobile breakpoints. Grid items must not simply overlap, vanish, or be clipped. Changing Item Size scales saved coordinates and spans together so the cards keep their physical size.

### 1.3 Deterministic reflow when capacity changes

Changing workspace dimensions might make some saved coordinates out of bounds or leave a widget too large. Reconcile the layout **transactionally**:

1. Calculate the new grid geometry and occupancy.
2. Keep each item on its current page and coordinate where it still fits without overlapping.
3. Resolve displaced items in stable, documented order (original page order, then row/column, then ID) into the nearest available valid region, preferring the same page.
4. Overflow to following pages if necessary; create additional pages if needed. Never delete items or silently alter folder membership.
5. If a widget cannot fit on *any* page because its span exceeds the new row/column count, clamp its span to a supported minimum where possible; otherwise require a different cell size or provide a visible, recoverable layout warning. Never lose the widget.
6. Present the proposed reflow in Settings before confirming disruptive changes when practical; cancelling must restore the previous setting and layout.
7. Persist the chosen size and resolved coordinates. Undo can restore the previous settings/layout together.

Never use implicit CSS grid auto-placement as the source of truth. Visual layout must reflect explicit model positions.

## 2. Items and containers

**Home-screen item types:** `app`, `shortcut`, `file`, `folder`, `widget`.

**Containers:** one page grid, the dock, or a folder's ordered children. Each shortcut *instance* belongs to exactly one container. The same underlying source app may have multiple distinct shortcut instances only if explicitly added more than once.

- **App/shortcut/file:** one-cell top-level tile with icon and label; click invokes existing Orbit action.
- **Folder:** one-cell top-level tile with name and up to four mini-icons in a 2×2 preview. Children are ordered, not assigned outer grid coordinates.
- **Widget:** generally multi-cell tile, content rendered inside it; it cannot be put in a folder or dock in v1.
- **Add control:** the existing `Add` tile is launcher chrome or an action, not a normal user-owned app occupying an editable grid slot unless the app already intentionally models it that way.
- **Dock:** stable across pages; separate ordered positions; support the existing number of dock entries and capacity based on its width. Do **not** force a three-icon Android dock.

Top-level items should always store page ID, row, column, rowSpan, and columnSpan. App and folder spans default to 1×1. Widget spans use their widget definition and saved instance span.

## 3. Pointer and keyboard gestures (desktop only)

- **Single left click app**: open/launch. **Single left click folder**: open it. **Single click widget**: execute only its own supported action, if any; otherwise do nothing.
- **Left pointer down + movement ≥ 6 px**: begin drag immediately, without a long press. A small movement below threshold is still a click.
- **Left press and hold ~450 ms without moving**: optional Android-inspired icon lift with contextual menu; if movement begins, dismiss menu and continue the same drag. Keep this separate from the essential immediate desktop drag gesture.
- **Right click**: Orbit-custom context menu; do not lift item. Page items offer Edit, Pin or Unpin, Move to Page and Delete (including Widgets); deleting asks for confirmation. Empty page space offers Add item, Add page (up to 9 pages) and Delete page (with confirmation; unavailable on the final page). Prevent native browser context menu on launcher cards, but do not suppress OS menus outside the launcher.
- **Pointer while dragging**: item follows pointer with exact original grab offset, `grabbing` cursor, elevated z-index, subtle 1.08–1.12 scale and shadow. Original location shows a ghost/outline.
- **Release**: commit only if a valid target is armed. Invalid release animates back and does not modify state.
- **Escape while dragging**: cancel, restore source layout, remove all previews and temporary pages.
- **Horizontal trackpad gesture**: one horizontal page transition per deliberate gesture, not a page flip per wheel event. Ordinary vertical scrolling must not accidentally change pages.
- **Page dots**: click navigates. **Keyboard**: Enter to open focused item, Shift+F10 for menu, Escape to close/cancel, PageUp/PageDown or context-appropriate left/right to change pages. Provide `Move to...` menu commands for keyboard accessibility.
- **Drag on empty workspace background horizontally**: optional page-swipe behavior if it does not interfere with selection; not required for v1.
- No touch-only handling, haptics, Android navigation-bar gestures, or mobile layout work is required.

During a drag, pointer capture must keep it active across children, overlays and page transitions. Handle `pointercancel`, lost capture, window blur, and unmount safely; a release outside valid areas **cancels**, rather than committing to a stale last-valid cell.

## 4. Drag state machine and previews

Use explicit state, not scattered mouse handlers:

```text
IDLE -> PRESSED -> DRAGGING -> HOVER_PREVIEW -> DROP_RESOLUTION
                     |              |                 |
                     |              |                 +-> COMMIT -> ANIMATE -> IDLE
                     |              +-> EDGE_PAGE_PENDING
                     +-> CANCEL -> SNAP_BACK -> IDLE

Additional submodes: EMPTY_CELL, REORDER_EDGE, APP_CENTER_FOLDER,
EXISTING_FOLDER, WIDGET_PLACEMENT, WIDGET_RESIZE, DOCK_INSERT.
```

The dragged overlay is separate from the page so it stays under the cursor during page slide, folder closing, or reflow. Use transforms, requestAnimationFrame or the codebase's existing animation primitives; do not rerender/persist the entire layout on each pointermove.

For each pointer movement:

1. Convert pointer location to workspace-local coordinates; account for scroll/transform/scale.
2. Identify active container (open folder, dock, page, edge zone, outside).
3. Identify candidate target and intent **using both geometry and dwell/hysteresis**.
4. Compute temporary preview from the **unchanged drag-start layout** (not from previous preview), preventing cumulative drift.
5. Render placeholder(s) and neighboring movement; leave permanent data untouched.
6. On pointerup, validate and atomically commit the currently armed preview or cancel.

On moving to another target, clear the previous target's timer. Add a small hysteresis margin to prevent rapid switching between folder and reorder modes near boundaries.

## 5. THE CRITICAL RULE: app center creates folder, edges reorder

This rule is mandatory and takes precedence over simplistic 'drop on occupied cell' logic.

### 5.1 Geometric zones

For an **occupied app tile**, divide its interactive **icon region** (not the whole wide row/cell including the label) into:

- **CENTER / MERGE ZONE:** centered inner region, initially approximately **50–60% of the icon's width and height**, configurable. The pointer or dragged icon's center must enter it. This area means: **'Put these two apps in a folder.'**
- **OUTER EDGE / INSERTION ZONES:** remaining border region around the icon, plus well-defined left/right or before/after insertion lanes in the cell. This area means: **'Reorder the apps, do not merge.'**

Use local pointer coordinates within the target icon bounds. Keep the dragged icon visually below/above pointer as appropriate, but decide the active target consistently (e.g. icon center anchor for cell and pointer position for fine-grained zone), without cursor grab offset causing surprising outcomes.

### 5.2 Timers and previews

- Pointer enters **center of another app**: start `folderIntentDwell` (~450–550 ms). Until armed, show a subtle *possible folder* highlight, **not** a reordering insertion placeholder on the same target.
- Dwell completes while still centered: target app enlarges very slightly, translucent folder plate/outline appears around it; show a clear `Create folder` preview. On release now: folder is created.
- Pointer moves into **edge zone**: cancel folder timer and merge preview **immediately** (with a slight hysteresis buffer); arm/preview **reordering**, possibly after `reorderDwell` (~250–350 ms). On release with reorder armed: reorder, **never create a folder**.
- Pointer returns to center: cancel reorder intent and restart folder intent timer. Preview may not commit until one specific target is armed.
- Quick passing across an icon's center must NOT make a folder. Releasing on an unarmed center target must not arbitrarily reorder into the target: cancel or return to the last clearly displayed valid preview according to a consistent policy; recommended **cancel on ambiguous unarmed center**.

**Outcome table:**

| Dragged item | Target | Active region | Result |
|---|---|---|---|
| App | empty cell | any | place app in empty cell |
| App | other app | center, after dwell | create folder: `[targetApp, draggedApp]` |
| App | other app | outer edge/insertion, after preview | reorder by sparse-grid shift |
| App | existing folder | center, after dwell | append app to folder |
| App | existing folder | edge | reorder/move around folder |
| Folder | app or folder | any | move/reorder whole folder; never merge/nest |
| Widget | app/folder/widget | any | place/reposition footprint if possible; never folder |
| App/folder/widget | empty eligible region | any | direct placement, no neighbor motion |

Use different visual shapes/colors for merge plate vs insertion outline so users understand the result without reading tooltips. The **center-vs-edges distinction must be automated-tested** at pixel/geometry level.

## 6. Sparse-grid placement and occupied-cell reordering

Grid coordinates are stable and may contain holes:

```text
BEFORE: [ A ][ B ][   ][ C ]
Move B to empty cell 3:
AFTER:  [ A ][   ][ B ][ C ]    (A and C did not move)
```

**Fixed-item placement rule (2026-10-21 user override):** existing page items stay at their saved coordinates throughout dragging and dropping. This supersedes occupied-cell shifting and edge-reorder behavior elsewhere in this specification. Keep the dragged rectangle at its exact grab-offset position and ignore its source rectangle for collision checks. Empty-space drops move only the dragged item. An occupied or out-of-bounds placement is invalid and returns the dragged item to its origin; never push neighbors aside.

**Tile-overlap folder intent (2026-10-21 user override):** a dragged shortcut overlapping any visible part of another shortcut or folder starts the 500 ms folder dwell, regardless of pointer position within that target. This replaces the center-only merge rule throughout this specification. Exclude tile spacing, the source item, pinned items and widgets. Choose the greatest overlap area; retain the current target on equal overlap, otherwise use page order. Leaving overlap cancels intent; changing targets restarts the dwell. A quick release before the dwell completes cancels the drop. Dragged folders and widgets never merge. Neighbors remain fixed. Cancellation, cross-page ownership and dock reordering retain their existing behavior. Compute validation from the unchanged drag-start layout and commit only a valid placement.

## 7. Folders: complete lifecycle

### 7.1 Create

Drag **App A** over **App B's center**, keep inside until merge dwell arms, then release. Create a new folder at **B's target cell** with child order `[B, A]`. A's source cell becomes empty. Show merge animation: target scales/highlights, dragged icon shrinks toward it, folder icon settles in. Default label: `Folder` (renameable).

### 7.2 Add to existing folder

Drag app to folder **center** and dwell; highlight folder and show `Add to folder` preview. Release appends its instance to the folder; source position becomes empty. Mini preview updates (first four child icons, 2×2). Dragging over the folder's **edge** reorders the folder instead of inserting a child.

### 7.3 Open and close

Single-click folder to open an animated overlay anchored to the folder's screen position, with darkened/blurred backdrop, readable editable title and a compact grid of children. For desktop Orbit use a well-sized floating panel, not the video phone's full-screen scrim with fixed dp coordinates. Dismiss via backdrop click, Escape or close control. Reverse the transition on close. Dock remains mounted and visually behind the scrim.

### 7.4 Reorder child apps

Drag children within the open folder to insert into a new order. Siblings preview the reflow. Child ordering is compact/sequential, unlike sparse top-level pages. Press Escape to cancel and restore order.

### 7.5 Extract app without breaking drag

Start dragging a child within an open folder. Once it crosses the folder's defined exit boundary with intent to leave, **close/collapse the overlay without releasing pointer capture**. The same lifted icon continues to follow the cursor over the home screen, with active page targets. Releasing in a valid cell moves it from folder to page in one atomic operation. Cancelling restores it to its original folder position and reopens/restores the appropriate UI state if practical.

### 7.6 Rename, dissolve, move

- Click/edit folder title or choose `Rename` from context menu; Enter commits, Escape cancels. Unicode/Arabic labels are supported.
- A folder with **one** child after extraction automatically dissolves: replace folder tile at its coordinate with remaining app. A folder with **zero** children disappears. Make this policy explicit and test it.
- Drag a closed folder around the grid or across pages just like a 1×1 item. Contents remain intact. Nested folders are disabled in v1.
- Removing a folder through the context menu must be explicit and safe; do not delete actual underlying apps. Offer `Unpack folder` into valid page cells where possible, or remove *shortcuts* with confirmation.

## 8. Widgets (new, required)

### 8.1 Widget picker

In the existing `Add` flow add `Widgets`, opening a small **Widget Gallery** with preview tiles showing name, example content and initial cell footprint. Widgets may be built into Orbit or connect to external services. A service widget must use an explicit account connection or credential flow, keep credentials out of ordinary config and backups, perform network work off the UI thread, and leave the launcher responsive when the service is slow or unavailable. Show a recoverable error and allow retry. Do not execute arbitrary downloaded code as a widget.

**TickTick Tasks is the only widget.** Remove retired local widget instances when loading saved data. Preserve TickTick instances and all launchable shortcuts.

Fetch every API project, including NOTE lists, and the Inbox. Project data may omit older open tasks; supplement it with open tasks grouped by priority using the task filter endpoint. Today and Next 7 Days are calendar views over those tasks (including overdue tasks); their labels are built-in views, while custom list names and IDs come from the API. Inbox (all tasks) is the aggregate view requested by the user. Keep credentials in the operating system credential store.

TickTick has its own background request state. Refresh Tasks fetches all lists; Refresh List fetches the selected project (aggregate/date views refresh their source lists). Show skeletons while refreshing, disable duplicate requests, and keep launcher input available. Mutations update the in-memory view immediately, serialize pending TickTick writes, reconcile creation IDs with the server, and restore the previous snapshot on failure. Show success/error toasts and play a short sound on completion. Editors consume clicks inside their panel and retain drafts on failed saves.

A task can be edited, moved to another list and deleted from the widget. The task editor shows Title, List, Notes, Priority, Tags, Due date and a Delete action. The List control selects the task's list, and saving with a different list moves the task there. The TickTick API rejects a task update that changes its list, so a move is performed as a copy into the new list followed by deleting the original, carrying over every other field (priority, tags, due date, start date, checklist, time zone) and adopting the new task's ID. The TickTick Focus Task widget selects the first unfinished task from Today, including overdue tasks, ordered by highest priority then oldest displayed date (due date, falling back to start date). Task views use this same date ordering, placing undated tasks last within each priority. The focus widget shows the task title, list, priority, due date, tags and a scrollable view of all remaining available task details, including notes, description, checklist, schedule, recurrence, reminders, status and time zone. New instances use a 4×2 footprint. Clicking the task title opens its editor, and its checkbox completes the task.

### 8.2 Widget placement

Select a widget in the gallery -> enter grid placement mode -> show its multi-cell outline -> pointer movement snaps outline to valid aligned cells -> click/drop to commit. Or provide `Place automatically` using the first valid rectangle on current page, adding a new page if required. Escape cancels. Reject collisions and out-of-bounds footprints visibly.

Widget cells are 1 px each. A widget’s visible default size is its design factor multiplied by Item Size (2×2 means 200×200 px at Item Size 100; 4×1 means 400×100 px); its saved footprint adds the configured Item Gap. Resizing changes width or height in 1 px increments.

### 8.3 Widget dragging and resizing

- Drag by widget header/handle or by the card surface outside interactive content. Show a footprint-sized ghost, not a 1×1 app ghost.
- Move to empty matching footprint without changing other items.
- In `Edit layout` / `Resize` mode show a bottom-right resize handle; dragging it snaps to whole-cell spans, with min/max configured per widget type. Preview valid cells; overlapping/out-of-bounds resize is rejected (or resolved only by a clearly previewed transaction). Persist on release.
- Ordinary text click should not unintentionally start drag; simple widget buttons, e.g. Refresh, remain clickable.
- No widgets inside folders or dock for v1. No nested widgets or spanning across page boundaries.

### 8.4 Visuals

Use Orbit's dark/purple rounded-card styling. Text wraps, clips gracefully, and scales with available widget card area while remaining readable; larger widget spans reveal more text. The gallery thumbnail and placed widget should match. Widgets receive their own context menu: `Refresh text` (where supported), `Resize`, `Move to Page`, `Remove widget`.

## 9. Pages, slider and cross-page dragging

Each page is an independent sparse grid. Keep the existing dots, active indicator, and `+` create-page control. The dock and header remain fixed while only page content slides horizontally.

- **Navigation:** click dots, horizontal trackpad swipe, appropriate keyboard shortcuts or existing arrow controls. Animate one page at a time (~250–350 ms), using gesture throttling and snapping. The user should never land halfway between pages.
- **Add page:** `+` creates a blank page and optionally switches to it. Page overview/reorder can be a secondary feature.
- **Remove page:** allow explicit removal when empty (always keep at least one), optionally clean up transient blank pages; do not automatically delete intentional empty pages without a defined user preference.
- **Cross-page drag:** when carrying an item, hover within ~40–60 px of left/right workspace boundary for ~500–700 ms. Show an edge cue/countdown; then slide to neighbor page while the dragged item remains attached to the cursor. Repeat for multiple pages, with delay/reset to prevent accidental skipping. Drop on that page uses normal validation.
- **Past last page:** show a temporary empty page on right-edge dwell. Only persist it if a valid drop succeeds. Cancelling leaves page count unchanged. Optionally support equivalent left-edge behavior if product requirements allow prepending pages.
- **No ghost duplication:** source stays logically owned by origin until destination commits; do not create duplicate items during animation.
- Page indicator updates when navigation completes. Only page content slides during page navigation; the persistent dock stays in place and scrolls horizontally when its contents overflow.

## 10. Persistent bottom dock

The existing Orbit dock is an independent persistent placement region shared by all pages. Its layout should reflect the current Orbit UI rather than a fixed three-icon phone dock.

- Show up to five Recent items and ten Pinned items. Keep the dock within the available screen width and horizontally scroll overflowing items without vertical scrolling.

- Drag app/folder into dock: highlight dock and show a clear insertion gap. Other dock items shift in preview; commit order and source ownership atomically.
- Drag dock item to page: show page cell target; move it out of dock on commit.
- Drag inside dock: reorder with live insertion indicators; no accidental launcher click on release.
- Dock capacity derives from visual width/slot size or a configured limit. If full, display invalid preview and reject, not replace an existing item.
- Widgets are not allowed in dock. Existing Orbit special/system controls can remain pinned/non-draggable if they are deliberately not user-owned shortcuts.
- If supporting duplicate shortcuts is desirable, make `Copy shortcut` an explicit action instead of implicitly duplicating on every dock drop.

## 11. Context menus, app actions and adding items

### Search Prefixes and Actions

Search anything supports named Prefix rules with a Name, Token and selected Action. Press Tab or Space after typing a matching Token to activate it, hide results and display its Name as a badge. Enter dispatches the remaining search text (without the Token) to that Action. With no results, Enter searches the query using the web search engine selected in General Settings (Perplexity by default). Saved page items appear after other matches. Search handling stays generic: token matching, badge state and Action dispatch only. Action-specific parsing belongs to the Action handler.

Settings has separate `Prefixes` and `Actions` tabs. A Prefix Token selects an Action; one Action can have multiple Tokens. The Actions editor shows and edits a comma-separated Token list, description, Action name and source. The Prefixes tab groups Tokens by Action and can add a separate Prefix. Actions are named reusable URL templates, TickTick task creation, or user-authored shell scripts. URL templates substitute `%s` with safely percent-encoded search text. Shell scripts receive search text as a separate argument (`$1` on Arch Linux; `$OrbitQuery` on Windows); never interpolate it into script source. Persist both collections in the existing config with defaults and migration that preserve existing settings.

The Settings panel keeps its title and tab controls visible while the active tab content scrolls vertically. Back remains available below the scrolling content. Shortcuts groups bindings by Action in a card grid; Prefixes displays each Name, Token and selected Action in an editable card grid.

Default prefixes: YouTube `y` or `غ`, ChatGPT `g` or `ل`, Image Search `i` or `ه`, Perplexity `p` or `ح`, Claude `c` or `ؤ`, Wikipedia `w` or `ص`, GitHub `gh`, Google Maps `m`, Translate ar to en `ف`, Translate en to ar `tr`, and TickTick `t`. Web search engines are Brave `:br`, Google `:g`, DuckDuckGo `:d`, Qwant `:q`, Bing `:b`, Startpage `:sp` and Ecosia `:e`. TickTick parses task title plus `!today`, `!high`, `!medium`, `!low`, `!none`, `#tag`, and `^List` or `~List` selectors inside its Action handler. Use Inbox when no list is specified and preserve Orbit's existing TickTick credential and API integration.

### Apps/files/shortcuts

Single click performs the existing Orbit open action. Right click displays relevant custom menu (e.g. `Open`, `Move to Page`, `Add to Dock`, `Remove from Home`, `Details`, `Rename shortcut` where supported). If a source app advertises quick actions, display them above the core actions in a small panel. Long press may show the same menu and lift the icon, emulating the reference recording, but immediate mouse dragging remains essential.

**Never implement 'Uninstall' by simply deleting a launcher icon.** Android's recorded popup shows `App Info / Uninstall`, but Orbit should distinguish shortcut removal from operating-system uninstall. For v1 use `Remove from Home`; optionally show `App Info` only if reliable metadata is available.

### Add menu

The approved 2026-10-21 entry point replaces all page Add tiles: right-click empty page space, then choose `Add item` from a context menu. Open the existing sheet with App selected; new items use automatic free-space placement, while widgets retain their existing placement flow. There is no top-bar Add button; the page-dot + still adds a page. Add tabs or sections: `Apps`, `Websites/Shortcuts`, `Files`, `Folders` (explicit new empty folder if desired), and `Widgets`. Selecting an item lets the user choose an explicit free grid cell or an automatic valid placement. Opening the Add picker must not reorder the existing page by itself.

### Outside and cancellation

Escape closes menus/overlays or cancels active drag, in that order. An invalid drop outside Orbit workspace/dock/folder returns to the origin. Clicking blank wallpaper closes context menu/folder. App launch must not fire after a completed drag.

## 12. Data model and invariants

A suggested normalized model (adapt to the project's existing stack):

```ts
type ItemKind = 'app' | 'file' | 'shortcut' | 'folder' | 'widget';
type Container =
  | { kind: 'page'; pageId: string; row: number; col: number; rowSpan: number; colSpan: number }
  | { kind: 'dock'; index: number }
  | { kind: 'folder'; folderId: string; index: number };

interface HomeItem {
  id: string;                         // unique shortcut / widget INSTANCE ID
  kind: ItemKind;
  title: string;
  icon?: string;
  sourceId?: string;                   // optional app/file target ID
  container: Container;                // exactly ONE location
  children?: string[];                // for folders; ordered IDs
  widget?: {
    definitionId: string;              // e.g. 'random-quote'
    text: string;                      // persistent sampled text
    seed?: string;
  };
}

interface HomePage { id: string; order: number; title?: string }
interface LauncherSettings { gridCellSizePx: number; gridGapPx: number }
interface LauncherState {
  schemaVersion: number;
  pages: HomePage[];
  itemsById: Record<string, HomeItem>;
  settings: LauncherSettings;
  activePageId: string;
}
```

Invariants:

1. All item IDs unique; every item has exactly one location.
2. A folder's children point to items whose `container.kind === 'folder'` and `folderId` matches; children do not also occupy page/dock slots.
3. Page top-level occupied rectangles do not overlap and stay within bounds.
4. App/folder = 1×1 in v1; widgets use legal integer spans; no widget inside folder/dock.
5. Nested folders are forbidden; no cycles.
6. Dock indices form a valid order and are capacity-limited.
7. Page IDs and item references must exist.
8. Every operation is transactional; the drag-start layout is a stable snapshot; previews are derived.
9. Maintain separate user-facing shortcut instance IDs versus actual app/file targets.

Save on **commit**, not on pointer motion; persist pages, active page, item positions, folder names/children, dock order, widget instances and text, settings. Include data version/migration and corrupted-state recovery. Use existing Orbit storage; do not impose localStorage if the codebase already has desktop storage/DB. Offer undo/redo for organization actions, including cell-size reflow if feasible.

## 13. Animation, geometry and performance

Suggested tuneable durations (not exact measurements):

- Icon hover 100–140 ms; pressed 80 ms.
- Lift 100–160 ms; scale ~1.08–1.12, shadow, label optionally hidden.
- Neighbor reflow 180–250 ms.
- Drop settle/snapback 150–260 ms spring.
- Folder create 200–350 ms; open/close ~220–300 ms.
- Page slide 250–350 ms.
- Popup fade 120–160 ms.

Show separate placeholders for empty-cell move, reorder insertion, center-folder merge, folder addition, dock insertion, and widget-size footprint. Do not rely solely on color to communicate state. Respect reduced-motion OS preferences where accessible.

Use Pointer Events with capture rather than native HTML5 drag-and-drop, which does not provide the desired fine-grained cross-page and hover interactions. Reuse project libraries where they allow custom sensors/collision geometry/overlays; avoid an unnecessary rewrite. Prefer CSS transforms and throttled geometry reads to achieve 60 FPS on typical desktops. On window resize, settings change, folder close, or lost pointer capture, never leave an orphan drag overlay.

### Desktop-specific edge cases

- High-DPI display scaling and browser zoom, if using a browser shell.
- Left-to-right grid coordinates must remain consistent even if labels are Arabic/RTL; RTL text must remain readable.
- Search focus, header controls, dock, and widgets should not trigger page-swipe gestures accidentally.
- Opening another window or losing app focus during drag must cancel safely.
- Context menu should be clamped inside available workspace; no cut-off menus by edges.

## 14. Acceptance tests for Codex

Write automated tests for **pure layout functions** and interaction/e2e tests for pointer gestures. The following are mandatory:

1. **Empty spot:** drag B `[A,B,_,C]` to index 2 -> `[A,_,B,C]`; A and C retain exact coordinates.
2. **Reorder forward:** occupied edge target shifts only the contiguous chain to the first following hole; unaffected items retain coordinates.
3. **Reorder backward:** if no hole follows, shift backward toward valid preceding hole; crossing row boundary works.
4. **Preview purity:** hover A, then B, then A again -> preview matches first A preview, no cumulative drift; Escape restores exact initial layout.
5. **Center-vs-edge (critical):** holding app over target app **center** and releasing after dwell creates `[target, dragged]` folder. Holding over **edge** and releasing reorders with no folder. Passing through center quickly creates no folder. Moving center -> edge cancels merge. Moving edge -> center cancels reorder and starts merge dwell.
6. **Existing folder zones:** app centered over folder adds it; app at folder edge reorders the folder position instead. Folder-on-folder never nests.
7. **Folder round trip:** create folder -> add third app -> open -> drag third app outside -> folder closes during same drag -> drop on empty cell -> folder still has two children.
8. **Dissolution:** extract one of two folder children -> last child replaces folder in its original cell.
9. **Cross-page:** dragged app remains attached while page slides; valid drop transfers ownership; cancel returns to source. Holding beyond last page and cancelling leaves no new page.
10. **Dock:** reorder, move into and out of dock, reject full dock, no duplicate shortcut instances.
11. **Settings size and gap:** change Item Size or Item Gap -> app/folder/widget spans and coordinates preserve their proportions; grid hit testing remains correct; overflow is reflowed safely; revert restores prior arrangement where practical.
12. **Widgets:** gallery lists default types; choose Random Quote; place 2×2; text persists across rerenders/restart; refresh only changes the intended instance; move and resize snap to whole cells; collisions rejected; widget cannot merge into folder.
13. **Persistence:** reload and get exact saved pages, holes, folder contents, dock, widget text/spans, and size setting.
14. **Click vs drag:** small pointer movement opens app once; drag never accidentally opens app.
15. **Safety:** Remove from Home never uninstalls app or deletes source file; invalid releases/blur/Escape never lose an instance.
16. **No overlap:** for thousands of randomized legal moves, all state invariants hold.

### Optional replay scenarios from the recording

Use the attached Claude timeline as **reference test fixtures for 1×1 sparse-grid mechanics**, not as desktop layout requirements. Notable sequences: a forward shift reaching the first hole; moves into empty cells without neighbor motion; no-hole-after-target backward reflow; merge two apps into a folder; add a third; open and drag a child out while folder closes; move entire folder. Do not hardcode the video's 3×4 size, names, image pixels, or three-icon dock.

## 15. Codex execution instructions / definition of done

1. Inspect the **existing Orbit repository** first: identify Settings page, current card/grid rendering, paging/slider, dock, item-open actions, context menu, storage and framework. Implement in this architecture without changing the established look.
2. Add central normalized launcher model and a pure, testable spatial grid engine. Integrate Settings-driven item size and gap with safe geometry recalculation.
3. Implement pointer drag state machine, empty placement, edge reorder, center-folder gesture and previews. **Do not call this done until the center-vs-edge tests pass.**
4. Add folder creation, folder overlay, child reordering and continuous extraction.
5. Add desktop page navigation, edge-hover cross-page move, temporary page creation, and dock operations.
6. Add Widget Gallery, local random-text widget definitions, placement and grid-span resizing.
7. Add Search Prefixes and Actions: edit Prefixes and Actions in Settings, activate a Token with Tab, dispatch the remaining search text, provide URL and shell Actions, and add TickTick task creation and the default site-search prefixes.
8. Add persistence, migration, undo/redo where feasible, accessibility actions, animation polish, and automated acceptance tests.
9. Demonstrate behavior in a running build. Report actual files changed, tests run, remaining limitations, and any differences from this spec.

**Definition of done:** I can customize item size in Settings, drag any app/folder to an exact grid cell and leave holes, *create a folder from the center and reorder from the edges of another app*, manage folder contents without interrupted drag, slide between pages while carrying an item, rearrange the persistent dock, choose/place/move/resize an elementary random-text widget, and use configured Search Prefixes to dispatch text to named Actions. Every successful change survives a restart. The result feels like a desktop-friendly Android launcher inside Orbit, not a sortable card list or a phone emulator.
