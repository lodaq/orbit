# V2 authority

The user approved `Orbit_Desktop_Launcher_UX_Spec_v2.md` on 2026-10-20 as authoritative wherever it conflicts with this document. Windows and Arch Linux, spatial grids, widgets, exclusive dock ownership, new UI terms, and the complete v2 build sequence are authorized. Existing visual artwork and nonconflicting performance and coding rules remain in force. The older build-step restriction does not apply to v2.

# Rules for the coding agent

These rules always apply. If a task seems to need breaking one, stop and ask.

## 1. Extremely fast and efficient

- Showing the launcher must feel instant. Budgets: visible within 50 ms (measured from the daemon receiving `orbit toggle` to the first frame on screen), and under 140 MB of memory while hidden and idle. The faster resident GPU renderer is preferred; do not trade away show time to meet the former 50 MB limit.
- When two options are possible (renderer, how to hide the window), measure both in step 1 and keep the faster one. Write the result in `spec.md`.
- Do work once and keep the result. Decode each icon once, when it is saved, and keep it ready. Never decode at launch.
- Never scan, fetch or read many files at startup or when showing the launcher. The Installed apps list is loaded from disk and is changed only by the Refresh installed apps button.
- Never block the interface. Disk scans and image decoding run on a background thread.
- No allocating or cloning in drawing or key-handling code where borrowing works.
- Write a file only when something changed. After a launch, the only write is the Recent list, and it happens after the app has started.
- Animation never delays input. Change the state first, then animate. Showing and hiding the launcher is never animated.
- Measure before optimizing. On the hot paths (show, hide, key handling, drawing), choose the fast option by default.

## 2. Minimal

- Write the least code that does the job: fewer lines, fewer files, fewer concepts.
- Prefer the standard library. Add a dependency only if it is small and saves real code, and write a one-line reason for it next to it in `Cargo.toml`.
- No abstraction until it is needed twice. No traits, generics, builders or options for a single use.
- Build only what the MVP needs. Do not build anything in `planned-features.md` or `roadmap.md`.
- Delete dead code. Never comment it out.

## 3. Simple

- Plain, readable Rust. No clever macros. No `unsafe` unless an OS call needs it, and then only inside a small function with a comment saying why.
- Short functions that do one thing (about 40 lines or less), with clear names.
- Comments explain why, not what.
- One obvious way to do each thing, used the same way everywhere.
- Use the words in `glossary.md` exactly: in the interface text, in code names and in comments (page, folder, shortcut, sheet, and so on).

## 4. When the rules conflict

Correct first, then simple, then minimal, then fast. The exception is the hot paths (show, hide, key handling, drawing), where fast wins once it has been measured.

## 5. Project rules

- The target is Arch Linux with Hyprland. Nothing else. Do not build anything for Windows (see `planned-features.md`).
- Code that talks to the OS (socket, window, `.desktop` files, icon themes) lives in one small module, so another OS can be added later. No `#[cfg]` and no abstraction for other OSes now.
- The interface is Slint with embedded Inter and the exact visual design in `doc/orbit-ui`; see SPEC section 10.
- New and updated UI follows `doc/design-system.md`: use shared `Theme` tokens for repeated visual values and shared controls for repeated patterns. Keep Orbit's approved appearance and migrate existing UI as it is touched.
- No Apple or Mac-only apps in default content, examples, tests or docs. The look is Apple-style. The apps are not.
- Orbit starts empty: no default pages, apps or folders.
- Saved files are written atomically (write a temporary file, then rename it). A missing file starts empty without a banner. A damaged file is renamed to `<name>.bad`, Orbit starts empty and shows a banner. A damaged file never crashes Orbit and is never overwritten.
- Launch apps directly, with no shell, and detached, so they survive a daemon restart. If the launch fails, stay open and show a banner.
- User-authored shell Actions are the explicit exception: pass the query as a distinct process argument (for POSIX shell, `$1`), never interpolate it into script text. Built-in URL and TickTick Actions must not invoke a shell.
- Orbit is mouse first. Every action must work with the mouse. Shortcut defaults are listed in `shortcuts.md`; users can manage action bindings in Settings. Handle them in one place and save them with the config. Ctrl+K focuses Search anything by default.
- Use Toast for action results and problems that are not tied to a visible field. Keep Banner text beside the field or form that needs correction. Background work reports through `toast` on the UI thread.
- `Orbit_Desktop_Launcher_UX_Spec_v2.md` is authoritative for desktop UX and build order. Existing `spec.md`, `glossary.md` and `shortcuts.md` provide compatible project rules and Orbit behavior. If the code and the docs disagree, ask. Do not guess. When a decision changes, update the doc in the same change.

Project filenames use kebab-case. Keep tool-required names such as `Cargo.toml`, `Cargo.lock` and `AGENTS.md`, and upstream license filenames. Rust module identifiers keep Rust syntax; a path attribute maps a kebab-case filename when needed.

## 6. Before finishing a task

- `cargo fmt` and `cargo clippy -- -D warnings` must pass.
- Write tests only for logic that has rules: the Recent list, page rules, folder rules. No tests for drawing.
- Report what changed in a few lines. Do not add extras nobody asked for.
