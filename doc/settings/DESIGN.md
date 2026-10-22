# Orbit settings: design spec

Redesign of the Orbit launcher's Settings screen. Five views, one consistent system. This spec is framework-agnostic: map it onto the app's existing components and styling approach rather than adding a new UI library. `tokens.css` / `tokens.json` hold every colour, radius and size. `reference/*.html` are standalone, pixel-accurate mockups (open in a browser; copy values from them, don't copy the markup blindly).

## Scope
- **Redesign only the settings panel** (sidebar + content). The Orbit top bar (logo, search pill, options and close buttons), the app dock and the bottom key hints are unchanged; keep the existing components.
- Keep all existing behaviour and data. Where this spec says "new", it is the only functional addition (excluded paths).
- Mockups are fixed-size boards (panel 1360×680; the full-screen one is 1440×1080). The real UI is fluid: sidebar fixed at 232px, content flexes, grids reflow.

## Views and navigation
Sidebar items: **General**, **Shortcuts**, **Search bar**, then **Back** (shows `Esc` hint) pinned to the bottom. Active item uses `--bg-nav-active` + white text.
**Search bar** has three sub-tabs in a segmented control: **General**, **Prefixes**, **Actions**. The old two-row button layout is gone; the page title is always an `h1` with a one-line description under it.

| View | Reference file |
|---|---|
| Search bar → General | `reference/search-bar-general.html` |
| General | `reference/general.html` |
| Shortcuts | `reference/shortcuts.html` |
| Search bar → Prefixes | `reference/search-bar-prefixes.html` |
| Search bar → Actions | `reference/search-bar-actions.html` |

## Design principles
1. Dark indigo, flat surfaces. No gradients, glows or shadows. Depth comes from 3 surface levels (panel → card → inset) and 1px borders.
2. One accent (`--primary`), used only for primary actions, switch-on and the slider. Danger is red text on a dark red fill, never solid red.
3. Every setting is a labelled control inside a card; every list is a grid of equal cards; every destructive action is a small danger-styled icon or button.
4. Text sizes: 24 page title / 22 "Settings" / 15 card title (500) / 14 controls and body / 13 descriptions and labels / 12 chips and kbd / 11 badges.

## Components
- **Panel**: `--bg-panel`, 1px `--border-panel`, radius 20. Sidebar 232px with right border; content padding 28px 40px, column gap 20px.
- **Nav item**: 44px high, radius 10, padding 0 12, 18px stroke icon + 14px/500 label, gap 12. Inactive text `--text-muted`.
- **Tab group**: track `--bg-track`, 1px border, radius 11, padding 4, gap 2. Tab 34px high, padding 0 18, radius 8, 13px/500. Active `--bg-tab-active` + white.
- **Card**: `--bg-card`, 1px `--border-card`, radius 14, padding 18–22.
- **Setting row**: title 15/500 + description 13 muted (3px gap) on the left, control on the right, padding 16 20, 1px divider between rows.
- **Switch** (`role="switch"`): 46×26, radius 13, thumb 20px white. On = `--primary`, thumb right. (Off style is not drawn; suggest `--border-control` track with `--text-muted` thumb.)
- **Text input / select**: 40px high, `--bg-inset`, 1px `--border-control`, radius 10, padding 0 14, 14px text. Select is native with a 16px chevron overlaid at right (pointer-events none), 220px wide.
- **Number stepper**: same box as input; value left, `−` and `+` 36px-wide buttons at right.
- **Slider**: native range, `accent-color: --primary`; label row above with current value (`120 px`) right-aligned in white/500.
- **Buttons**: 40px high, padding 0 18, radius 10, 14px/500. Default `--bg-control` + `--border-control`. Primary `--primary` fill, white text. Danger `--danger-*`. Small = 34px / 13px. Disabled (Save before changes) `--disabled-bg` / `--disabled-text`, no pointer.
- **Icon button**: 36×36, radius 9, 16px icon; default and danger variants. Always has an `aria-label`.
- **Dashed add button**: full width, 40px, 1px dashed `--border-dashed`, accent text, plus icon.
- **Path field**: monospace 13px, 36px high, `--bg-inset`, 1px `--border-panel`, radius 10, truncates.
- **Removable chip** (excluded paths): 30px high, mono 12.5px, `--bg-inset`, padding 0 6 0 12, radius 8, ×-button 20px. Wildcard chip (`.*`) is accent-coloured (`--text-accent`, `--border-dashed`).
- **Key chip** (shortcuts): 28px high, 12px, radius 7, × button 18px.
- **Status pill**: 24px high, radius 12, `--success-*`, 12px/500.
- **Type badge** (Actions): 11px/600, `--bg-nav-active` fill, `--text-kbd`, radius 6.
- **kbd hint**: 22px high, radius 6, `--bg-control`, 12px.
- **Icons**: inline stroke icons, 24px viewBox, stroke-width 1.8, round caps. Map to the app's icon set (Lucide equivalents: settings, keyboard, search, arrow-left, x, pencil, trash-2, plus, chevron-down, file, folder, ban). Logo is custom (unchanged).

## View specs

### Search bar → General
1. Header: "Search bar" + "Control how Orbit finds things and where it looks." Tab group (General active).
2. **Behaviour card** (two rows): *Smooth cursor* switch (on by default as today); *Fallback search engine* select (current value Perplexity) + Save.
3. **Search locations** (section title + description), two equal cards side by side:
   - *File search paths*: path field + edit + delete icon buttons per path; dashed "Add file path".
   - *Directory search paths*: same, "Add directory path".
4. **Excluded from file search** (NEW), full-width card: title, description ("Files and folders with these names are skipped. Use .* for anything that starts with a dot."), an input ("e.g. dist") + "Exclude" button at the right, and a wrapping row of removable chips.
   - Default list (16): `.git`, `node_modules`, `.*` (wildcard: any name starting with a dot), `.local`, `.claude`, `.agent`, `.cache`, `.venv`, `__pycache__`, `dist`, `build`, `.next`, `target`, `.idea`, `.vscode`, `.npm`.
   - Approved semantics: patterns match a single path segment (file or directory name), case-sensitive, `*` wildcard allowed, excluded directories are not descended into. Applies to file search only. Seed the defaults on first run; user edits persist. Ignore blank or duplicate entries.

### General
Two columns (≈1.3fr / 1fr), equal height.
- Left card **Home screen**: *Page margin (px)* and *Item gap (px)* steppers side by side; *Item size (px)* number stepper (80–176 px, 8 px increments) with live preview; "Reset to default" + primary "Save"; a live-preview strip at the bottom (64px app tile + "Live preview · Zen Browser" + "1880 columns × 828 rows. Save reflows displaced items; Back cancels unsaved sizing."). Keep the existing semantics: the preview updates live, Save commits and reflows, Back discards unsaved sizing.
- Right column: **TickTick account** card (title, "Connected" pill, *Personal API token* input with placeholder "Paste a replacement token", primary "Connect TickTick", danger "Disconnect", status line "Tasks refreshed"); **Data** card ("81 apps · Cached inventory", buttons Refresh installed apps / Export data / Import data).
- The pill and status text are driven by real state (Connected / Not connected, last refresh result).

### Shortcuts
Header "Shortcuts" + "Choose an action, then record its shortcut. Changes are saved automatically." One card, two-column grid of 56px cells (name 190px, key chips, small "Add" button), 1px dividers.
Actions in order: Focus Search anything, Close the top layer, Move left / previous page, Move right / next page, Move up, Move down, Open / Choose, Next field, Previous field, Open Settings, Add page, Delete page. Empty = italic "No shortcut assigned".
**Approved behavior:** shortcut chip labels are display-only. × requests removal and ✓ confirms it. Use Add to record a replacement after removal; there is no Change button or chip-label editing. Keep the existing recorder and all configured actions, including Delete previous word.

### Search bar → Prefixes
Header + tab group (Prefixes active). 3-column grid (gap 14) of compact cards: name (15/500) and a mono `token → action` line (token in bold white, accent colour for the rest), edit + delete icon buttons at right. Example cards: YouTube `y → YouTube`, ChatGPT `g → ChatGPT`, Brave `:br → Brave`, Google `:g → Google`. The mockup shows 12; render all, the content area scrolls. Include the existing Arabic-keyboard token variants as they are today (the mockup omits them for brevity).

### Search bar → Actions
Header + tab group (Actions active) with primary "New action" at the right of the tab row. 3-column grid of cards: name + type badge (URL, or TickTick for the special action), `Tokens · y` in mono, description clamped to 2 lines. Cards open the existing edit view on click or Enter (make the card a button/link, not a div with onClick). The mockup shows 9; render all, the content area scrolls.

## Layout and responsiveness
- Grids: `repeat(3, 1fr)` ≥ 1100px content width; 2 columns below; 1 column below 640px. Two-column splits (General, Shortcuts) collapse to one column below 900px.
- The content area scrolls inside the panel; the sidebar and tab group stay fixed. Never let the page body scroll sideways.
- Keep the panel inset from the screen edges as today (40px) and above the dock.

## Interaction and accessibility (not all drawn)
- Hover: raise controls one step (`--bg-control` → `--disabled-bg`; cards unchanged). Focus-visible: 2px ring `--icon-accent`, 2px offset, on every interactive element. Pressed: 1px darker fill.
- Esc goes Back one level (sub-tab → settings → desktop), as today.
- All text meets 4.5:1 on its surface (checked for the palette in `tokens.css`). Icon-only buttons need `aria-label`; switch uses `role="switch"` + `aria-checked`; nav and tabs use `aria-current="page"` for the active item.
- Don't convey danger by colour alone: Delete buttons keep their icon/label.

## Implementation notes for Codex
- Start from `tokens.css`; add the variables to the app's global stylesheet (or the equivalent theme object) and reference them everywhere. Do not hard-code hex values in components.
- Build the shared pieces first (Card, SettingRow, Switch, Select, Stepper, Button, IconButton, Chip, TabGroup, NavItem, Grid), then the five views.
- Compare each view to its `reference/*.html` at the board size and fix visible differences before moving on.
- Don't change behaviour or stored data formats, except adding the excluded-paths setting (persisted with the other search-bar settings and consumed by the file indexer/search).
