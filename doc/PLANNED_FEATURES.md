# Orbit planned features

Features you have decided to build after the MVP. None of them is built in the MVP, and the coding agent must not build them until they are moved into the MVP scope.

Ideas that are not decided yet are in `ROADMAP.md`.

## General

| Feature | Notes |
|---|---|
| Widgets | Not defined yet. |
| Search bar | Taken out of the MVP. The Figma design has one: a search field at the top of the launcher with a shortcut hint. What it would search (apps and folders only, or also installed apps) is not decided. |
| Emoji as icon | Taken out of the MVP. Needs a quick test first: Slint's default renderer may draw emoji as flat one-color shapes, and color emoji may need the Skia renderer, which would make Orbit bigger and slower to start. |
| URL as icon | Taken out of the MVP. Orbit would fetch a website's icon once, save it, and never fetch it again unless you ask. Needs a way to download and read web pages. Which library does it is the coding agent's choice, under `RULES.md`. |
| Image icon | Taken out of the MVP. Choose a PNG or SVG file in the Icon picker, copied into Orbit's icons folder when saved. A native file dialog takes focus, so hide-on-focus-loss must pause while it is open (or the file is chosen by typing its path). |
| Background image | Taken out of the MVP, which has only solid colors. "Choose image…" in Settings. The image is copied and resized to the screen once, because a decoded 4K image is about 33 MB. Same file dialog note as Image icon. |
| Custom and gradient backgrounds | Taken out of the MVP. A color of your own choosing, and soft layered gradients. |
| Frosted-glass dock | Needs a blurred copy of the background, made once when the background is set. Store only the strip behind the dock, not a second full-screen copy. Only useful together with background images or gradients. |
| Keyboard way to move items | Not for now. Move an item into a folder or onto the next page with shortcuts. In the MVP both are mouse only (drag and drop). |
| Terminal apps | Taken out of the MVP. The scanner skips `.desktop` entries with `Terminal=true`. Running them needs a terminal chosen in Settings, because `$TERMINAL` is often not set and each terminal takes `-e` differently. |

## Windows

Windows 11 is not a target in the MVP. Build all of this only when it is moved into scope. Testing is on a dual-boot machine.

| Feature | Notes |
|---|---|
| Windows build | Named pipe instead of the Unix socket. Config in `%APPDATA%\Orbit\`, not shared with Arch. One small OS module with the same functions as on Arch. |
| Toggle Orbit shortcut | The daemon registers a global shortcut with `RegisterHotKey`. Default `Alt+Space` (Windows only, works while Orbit is hidden). Settings has no Shortcuts section, so a place to change it is needed. If another program owns it, show a banner the next time the launcher opens with `orbit toggle`. |
| Autostart | A shortcut to `orbit.exe daemon` in `shell:startup`. An "Open at login" switch in Settings is an idea for later (it is in no list yet). |
| Windows installed apps | Read Start Menu shortcuts (`.lnk`). Store apps (`shell:AppsFolder\<AUMID>`) come after that. Extract each icon. Store app icons are the hardest part. |
| Launching on Windows | `.lnk` files and Store apps need the Windows shell API. The rule "no shell" must mean no `cmd.exe`, not "no shell API". |
| Windows window details | Hide the taskbar entry, switch off Windows' own show/hide fade, get foreground focus. These need small native calls. Check the speed budget again on Windows. |
| Windows packaging | Build notes and a README section. |

## Add more here

-
