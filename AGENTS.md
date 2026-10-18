# Orbit

Orbit is a fullscreen app launcher for Arch Linux (Hyprland), written in Rust and Slint.

Before any task, read these files in `doc/`:

- `SPEC.md` (what to build, and the build order)
- `RULES.md` (how to write the code)
- `GLOSSARY.md` (the only words allowed in the UI, code and comments)
- `SHORTCUTS.md` (the few keys Orbit has)

The rules, in short:

1. Extremely fast and efficient. Showing the launcher must be instant. Measure before you optimize. Do work once and keep the result.
2. Simple. Plain, readable Rust, short functions, comments that explain why. No clever tricks.
3. Minimal. Write as little code as possible. No abstraction until it is needed twice. Add a dependency only if it is small and saves real code, with a one-line reason in `Cargo.toml`.
4. Easy to understand file structure. One file per job, with a name that says the job. Keep the layout in `doc/STRUCTURE.md` up to date.
5. After every code or UI change, build the release binary and restart Orbit so the running launcher reflects the source.
6. Before finishing a UI change, open the rebuilt launcher and follow the real user path from its entry point through the changed control. Confirm the resulting content and state are visible in the running app; do not rely on source inspection, a build, or a manually opened intermediate state.

Never build anything from `doc/PLANNED_FEATURES.md` or `doc/ROADMAP.md`.
Build one step of `SPEC.md` section 13 at a time. Never start the next step.
If the code and the docs disagree, or a task seems to need breaking a rule, stop and ask.
Before finishing: `cargo fmt` and `cargo clippy -- -D warnings` must pass.
Report what changed in a few lines. Add nothing nobody asked for.
