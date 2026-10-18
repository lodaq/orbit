# Orbit roadmap (after the MVP)

Nothing in this file is in the MVP. The MVP is, on Arch Linux with Hyprland: pages, apps, folders, the dock (Recent and Pinned), Settings, shortcuts, and the icon picker.

Features you have decided to build are in `PLANNED_FEATURES.md`. This file is only for ideas that are not decided yet.

## Parked during design (suggested, not decided)

These came up while designing the MVP and were pushed out. Delete any you do not want.

| Idea | Why it was parked |
|---|---|
| Dock magnification on hover, like the macOS Dock | Needs per-frame animation driven by the mouse position. The MVP copies the Dock's shape and styling only. |
| Real frosted-glass blur behind the dock and sheets | Slint has no backdrop blur. The MVP dock is a plain semi-transparent bar. |
| Running-app indicator dots under dock icons | Needs Orbit to track which processes it started, and what happens to them afterwards. |
| Dark mode | The MVP ships one light, iOS-style theme. |
| Folder pages (swipe between pages inside a folder, like iOS) | The MVP uses one scrolling grid per folder. |
| Nested folders | The MVP keeps folders flat, like a phone. |
| Auto-tinted tile background taken from the icon | Apps have no color property in the MVP. Tiles are neutral, and finished artwork fills the tile. |
| Editable shortcuts, and more keys such as page, edit and delete keys | The MVP has a few fixed keys (arrows, Enter, Esc, Tab) and is made for the mouse. |
| Hyprland layer-shell mode | The MVP is a normal fullscreen window with a window rule. Layer-shell would need a different toolkit. |
| Choosing which monitor the launcher opens on | The MVP opens on the monitor with the cursor. |
