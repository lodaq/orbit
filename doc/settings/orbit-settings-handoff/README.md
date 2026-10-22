# Orbit settings: handoff package

| File | Purpose |
|---|---|
| `DESIGN.md` | The spec. Start here. Principles, components, per-view layout, behaviour, accessibility. |
| `tokens.css`, `tokens.json` | Every colour, radius and size as variables. |
| `reference/*.html` | Standalone mockups of the five views. Open in a browser (needs internet for the Inter font, otherwise falls back to system-ui). |

## Use with Codex
Copy this folder into the Orbit repo (for example `docs/settings-redesign/`), then give Codex this prompt:

> Implement the settings redesign described in `docs/settings-redesign/DESIGN.md`. Use the tokens in `tokens.css`, reuse the app's existing components and styling approach, and match the HTML files in `reference/`. Build the shared components first, then the five views. Keep existing behaviour and data formats; the only new feature is the "Excluded from file search" list. Ask me before changing anything that isn't covered in the spec.

If you use an `AGENTS.md`, add one line pointing to `docs/settings-redesign/DESIGN.md`.
