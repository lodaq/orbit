# Orbit Robots capability map

Status: implementation authorized on 2026-11-03. Steps 1–3 implement Robot data, its backup boundary, worker-side tools and the queued multi-turn runtime. Step 4 implements the workbench and runtime/backup coordination; its isolated Linux UI flow is verified. Step 5 (full backup/runtime release acceptance) remains next.

The first release supports persistent Robot profiles with one Robot executing at a time. Commands queue per Robot; hiding Orbit does not stop work. The existing AI Prefix remains independent. Saved Teams and delegation belong to a later release.

| Module id | Responsibility | Depends on |
|---|---|---|
| robot-data | Robot profiles, reusable Roles, project binding, model selection, installed Skill references, local stdio MCP configuration, conversations, activity, ratings and immutable starting model/Role snapshots | — |
| robot-tools | Per-action approval for local file and shell operations, installed Skill loading, enabled MCP tool discovery/calls, built-in web research, keyring credential references | robot-data |
| robot-runtime | Multi-turn Ollama/OpenAI/Gemini requests, command queues, one active Robot, approval suspension, tool/result recording and continued work while hidden | robot-data, robot-tools |
| robot-workbench | Add item entry, shared 2D Robot illustration, Canvas opening, Settings management, conversation browsing/deletion/rating, progress, activity, approvals and results | robot-data, robot-runtime |
| robot-backup | Versioned export/import of profiles, Roles, tool configuration, conversation history and ratings, without credential values | robot-data, robot-runtime |
| robot-teams | Saved lead/specialist Teams, direct assignment, delegation, sequential shared-project execution and one combined history per assignment | robot-runtime, robot-workbench, robot-backup |

Build order: robot-data, robot-tools, robot-runtime, robot-workbench, robot-backup. All five are required for the first release. Implement robot-teams only in the later step requested by the user. These are capability boundaries, not a requirement to introduce a separate abstraction or crate for each row.

## Existing integration points

- `src/config.rs`, `src/apps.rs`, `src/add-item.rs`, `src/item-drop.rs` and `src/spatial.rs`: Robot item identity, placement, opening, validation and exhaustive item handling. Preserve current Canvas changes and layout locking.
- `src/ai.rs`: existing provider settings, HTTP behavior and OS keyring conventions. Reuse applicable behavior without changing the independent single-question AI Prefix.
- `src/daemon.rs`, `src/main.rs` and `src/toast.rs`: resident execution and UI-thread result delivery; showing Orbit must not trigger disk scans or network requests.
- `ui/launcher.slint`, `ui/controls.slint`, `ui/theme.slint`, `src/shortcuts.rs` and `src/text-direction.rs`: workbench, Settings, approval controls, top-layer navigation and first-strong Arabic direction.
- `src/backup.rs` and `src/storage.rs`: validate and stage imports, retain previous data and obtain a consistent Robot snapshot while workers exist. Imported history must never replay actions automatically.
- `doc/spec.md`, `doc/glossary.md`, `doc/shortcuts.md`, `doc/structure.md` and a new execution/permission ADR: document the approved behavior and exceptions to existing one-question AI and shell-launch rules.
- Rust unit tests and an isolated UI path alongside `tests/canvas-ui.py`: permission, queue, persistence and backup rules plus real Add item → Robot → command → approval → result interaction.

## Decisions for the module specifications

- Interpret “one Robot at a time” as one executing Robot, while allowing multiple saved profiles. Define queue ordering across profiles and explicit restart/interruption behavior; never silently repeat a possibly completed local action.
- Snapshot the provider/model and Role instructions at conversation creation. Later profile or Role edits must not rewrite resumed conversations.
- Treat the project folder as the working directory, not a sandbox. Show the exact local operation, path/working directory and arguments before approval; bind approval to that operation only. Skill instructions cannot grant approval.
- Enabling an MCP server/tool grants automatic execution authority, including any machine access exposed by that server. Explain this in configuration. Store credential values only in the OS keyring, including MCP secret environment values; persist references only.
- Choose a built-in search service that works with all three model providers, including Ollama. Existing browser search URL templates do not provide research results to a Robot. Preserve returned source URLs and require approval before saving a requested Markdown report.
- Define Skill compatibility explicitly: Orbit executes supported workflows using its own tools, not Codex/T3-specific tool names or an external agent process. Unsupported workflow requirements must produce an actionable error.
- Specify deletion while commands are queued/running, backup consistency, missing projects/Skills/servers after import, and missing keyring credentials. Never restore automatic execution authority from a backup without an explicit user enable action.

## Release acceptance

Use the user's acceptance checks as the release gate: saved Robot/Role/model/Skill/MCP settings; queued multi-turn work and local approvals; automatic enabled MCP calls; background completion; sourced Markdown research; history and five-star ratings; credential-free backup round trips; Canvas and Arabic interaction; release build, formatting and Clippy. Test UI input only on an isolated display and profile, then restart the normal daemon hidden.

Permission tests must prove that denied, stale or absent approval causes no local tool execution. Backup tests must inspect serialized output for seeded credential values. Provider and MCP fixture tests cover tool calls, errors and follow-up turns; live provider checks require configured credentials and must be reported separately from fixtures.

Teams acceptance remains deferred: direct assignment, lead delegation, sequential edits in one shared project folder and one combined team history per assignment.

Workbench verification (2026-11-03): private Xvfb display and fresh profile, Add item → Robot → saved Role/project/model/Skill/MCP → two queued commands → Skill/write approvals → automatic MCP call → linked Markdown report → hidden execution → history/rating after restart → permanent deletion. Arabic-leading and mixed text were inspected. Model and research responses were local fixtures, not live provider/search checks. Artifacts: `target/ui-check/canvas-zb0ug6mf/`.
