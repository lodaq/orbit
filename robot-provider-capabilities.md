# Robot provider accounts

2026-11-05: the user authorized a shared second Settings tab named Providers, reuse of all existing T3 Code provider configurations/authentication, and completion of connection and execution together. This supersedes the one-step restriction for this request. The shared Providers section in `doc/spec.md` defines source references, immutable connection fingerprints and native permission scopes. Current implementation includes shared provider import/model selection and direct Codex app-server, ACP and Pi RPC execution. Formatting, Clippy with warnings denied, the release build and 180 normal tests passed (6 opt-in checks ignored). Isolated release verification followed Settings → Providers → Robot selection → saved profile → native command → command/action permissions → visible Arabic reply → restart and reopen; wrapped activity text has an explicit width before height measurement so replies remain reachable. Evidence: `target/ui-check/canvas-ice3igfa/`. Separate live checks returned expected answers for 9 of 17 enabled accounts; the other 8 reported protocol/model errors or timeouts, so complete live readiness is not claimed. Details: `target/ui-check/providers-verification.md`. Windows and other model choices remain unverified.

Requested on 2026-11-03: add Ollama, OpenCode, Antigravity, GitHub Copilot and Codex/ChatGPT to Orbit Robots, allowing multiple named instances/accounts of each provider. This extends Robots; the independent AI Prefix keeps its existing behavior.

## Approved capability map

| Module id | Responsibility | Depends on |
|---|---|---|
| robot-provider-accounts | Persistent named provider instances, account-specific connection/configuration, Robot selection, immutable conversation instance binding, validated backups without credentials | Existing robot-data and robot-backup |
| robot-provider-connections | Provider-specific setup, sign-in/status and model discovery; separate authentication/configuration per instance | robot-provider-accounts |
| robot-provider-execution | Native provider sessions, replies/activity, tool approval routing, cancellation and restart recovery | robot-provider-connections and existing robot-runtime |

Build order: robot-provider-accounts → robot-provider-connections → robot-provider-execution. These are capability boundaries within existing modules, not a requirement for new crates or dependencies. Implement one approved step at a time.

## Required behavior

- A provider is a driver; an instance is a named account/connection. Multiple instances may use the same driver. Each Robot selects one instance and a model.
- Ollama instances select separate loopback endpoints. Signed-in providers require independent supported authentication/configuration locations, not global account switching.
- New conversations bind to the selected instance. Editing a Robot or selecting another account must not retarget existing conversations. Missing/deleted/unavailable instances report an error; no silent fallback.
- Keep existing OpenAI API and Gemini profiles and conversations readable. Codex/ChatGPT is distinct from the existing separately billed OpenAI API provider.
- Secrets, refresh tokens and provider authentication files are never included in config, conversation activity or exported backups. Restored account references require reconnection.
- Setup and model discovery are explicit background operations. Never launch agents, scan accounts or access the network when showing Orbit.
- Do not sign out, replace or modify T3 Code accounts. New Orbit instances use their own supported authentication locations.
- Existing persisted unfinished work remains Interrupted after restart/import, without replay.

## Approved execution boundary

The current execution ADR says Orbit owns all model/tool execution and does not spawn Codex or T3 Code. Supporting native signed-in agents requires revising that restriction for provider subprocesses; Orbit must never depend on the T3 Code application.

The installed T3 Code adapters inspected on 2026-11-03 use Codex app-server, Antigravity ACP and OpenCode server interfaces. Account isolation and GitHub Copilot integration must be verified against each provider interface before implementation. T3 Code implementation details are reference evidence, not code to copy or a stable public API contract.

Native agents execute their own tools. Orbit must map provider approval requests to the owning conversation and exact action, preserve denial/cancellation, and display the actual permission scope. Do not claim Orbit per-action approval for operations the provider cannot expose. Unsupported permission behavior must be resolved before enabling that driver, never replaced with automatic approval.

## Acceptance gates

1. Save/reopen two instances of the same provider; assign them to separate Robots and verify independent credentials and session state.
2. Verify each requested driver with its documented interface: setup, model selection, response, tool approval/denial and cancellation. Missing installations and expired login show actionable errors.
3. Switch a Robot instance and prove an existing conversation retains its original account. Test instance deletion without account fallback or credential destruction.
4. Round-trip old and new backups without secrets or execution authority. Existing Ollama/OpenAI/Gemini conversations remain readable.
5. Follow Add item → Robot → provider/account selection → save → reopen → submit on an isolated display and test profile, including Arabic/English text. Do not manipulate the active desktop.
6. Pass cargo fmt, targeted tests, cargo clippy -- -D warnings and cargo build --release. Restart the normal Orbit daemon hidden after source changes.

Status: step 1 (provider accounts) is implemented and verified. Steps 2–3 (native connection/sign-in and execution) remain next.

Step 1 verification (2026-11-04): formatting, Clippy with warnings denied, release build and the complete test suite passed (172 tests; 5 existing opt-in tests ignored). Isolated display/profile checks followed Add item → Robot, saved two Ollama accounts using independent local HTTP fixtures, verified replies from both, switched the Robot account, resumed the original conversation without retargeting, and reopened saved accounts after restart. Arabic and mixed-text account fields were inspected. A separate fresh isolated run verified Codex/ChatGPT metadata selection, the account name in the workbench summary, restart persistence and an explicit unavailable error without any tool execution. The normal release daemon was restarted hidden. Evidence: `target/ui-check/canvas-exj9wroj/accounts-report.json` and `target/ui-check/canvas-k_sk7zzd/report.json`, with adjacent screenshots. Live cloud account calls, native sign-in/execution and Windows are not verified in this step.
