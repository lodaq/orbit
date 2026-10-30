# Tasks: Reliable English assistance

Plan: `tasks/plan.md`. Status: implementation complete after user approval; verification results and explicit limits follow. Preserve all pre-existing uncommitted work. These tasks are sequential.

For every implementation task, run its focused checks, build `cargo build --release` after the code/UI batch, verify affected behavior using an isolated compositor/profile, and restart the normal daemon hidden. Never direct input or focus changes at the user's active desktop. Update this checklist with actual evidence, not assumed results.

## Completion evidence (2026-10-30)

All four implementation tasks are complete. The detailed checkboxes below retain the original planned verification breadth; they are not a claim that every combined criterion received an independent end-to-end probe.

- Final `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` and `cargo build --release` passed. Tests: 127 passed, six existing live-account tests ignored. Logs are under `target/english-assistance/repair-*`.
- Regression tests cover source-aware preview/acceptance, rejected unrelated words, prompt migration/custom preservation, bounded requests, stale state and independent errors. A failing real pointer-acceptance test reproduced the focus bug before the shared control fix.
- Final release was tested on Xvfb `:97` with an isolated profile/runtime and Hyprland IPC fixture after the headless compositor could not start. All input and screenshots stayed on that display. Settings → Search bar → English assistance → Back → search was exercised.
- Physical Ctrl+E, Tab and mouse Accept word produced the displayed completion with native Undo/Redo. Grammar `go → goes`, suggestion replacement, Ahmed/number/Friday/question preservation, wrapped text, Arabic and mixed text were checked in the running app. Screenshots and the harness are under `target/english-assistance/isolated/`.
- Live local Qwen3.5 and deterministic fault replies were both exercised. Partial/malformed/missing-model/offline responses preserved the query. Retry cleared the error while retaining a valid sibling completion. The timeout probe captured a pending request, so timeout recovery is **not** claimed as a completed UI check.
- Normal settings now select local `qwen3.5:2b`; known built-ins migrate on load. Unrelated saved settings were compared and preserved. The final daemon was restarted hidden, and isolated app/display processes were stopped. Rollback binary/settings remain under `target/english-assistance/repair-backup/`.
- Native Hyprland rendering, launcher show/RSS budgets, every combined cancellation/Prefix scenario and exhaustive model quality were not remeasured. Existing regression coverage remains; model wording can still be imperfect.

## Task 1: Establish the selected model and migrate built-in prompts safely

**Description:** Use installed `qwen3.5:2b` with thinking disabled as the single shared assistance model. Finalize the three prompt contracts against fixed cases, then update built-in prompts and their existing load-time migration. Keep the current JSON response keys and saved settings shape. Make the current word and its preceding context explicit to completion requests. Do not switch the normal profile until Task 4 verifies the rebuilt application.

**Acceptance criteria:**
- [ ] Live fixed cases return useful contextual completions (`impro`, `concen`, `rep`, a completed word, an obvious typo and a trailing-space next word); usable rewrites of `I want improve my English.` and `Can you helps Ahmed prepare 3 reports by Friday?`; and correct agreement feedback for `She go to school every day.`. Report raw-model failures separately from rejected outputs. Record cold/warm latency and verify maximum eligible input fits the chosen context without silent truncation.
- [ ] Known default Suggestions, Autocomplete and Grammar prompts migrate exactly once, including both known old completion versions. A second migration is a no-op. Custom prompts, explicit model selection, endpoint, feature switches, shortcuts, AI settings and unrelated config values are preserved; a save failure does not discard saved settings.
- [ ] Keep local-only model/endpoint checks, bounded schemas/responses and background warming. The model and prompts survive isolated restart through the existing settings/save path; no new dependency or config version is introduced.

**Verification:**
- [ ] `cargo test --bin orbit english_assistance::tests` and `cargo test --bin orbit config::tests` pass with meaningful migration regressions.
- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo build --release` pass.
- [ ] In the isolated rebuilt launcher, open Settings → Search bar → English assistance, select the target model, test/save prompts, return to search, observe real results, restart the isolated instance and confirm persistence. Retain prechange binary/settings for rollback; restart the normal daemon hidden after verification.

**Dependencies:** None.

**Files likely touched:**
- `src/english-assistance.rs`
- `src/config.rs`
- `doc/english-assistance.md`

**Estimated scope:** Medium: 3 files.

## Task 2: Make autocomplete preview and acceptance describe the same edit

**Description:** Fix the shared completion path from validated model output through publication, rendering and acceptance. Keep the replacement word/range separate from the visible suffix. Preserve current-word spelling corrections with an explicit replacement preview. Reuse the native text-editing bridge and the existing shortcut callback.

**Acceptance criteria:**
- [ ] `I need to impro` displays only muted `ve` and accepts `I need to improve`; a single partial word also works. Trailing whitespace appends one next word, unchanged complete words show no completion, and typo replacements display the full intended change. Unrelated first-word echoes and `predict` for an unrelated fragment are rejected; `predic` → `predict` remains valid.
- [ ] Physical Ctrl+E, Tab and Accept word apply exactly the same displayed edit without submitting. One Undo restores the query and Redo reapplies the edit. Exact Prefix Tokens retain Tab priority, and the Prefix badge survives acceptance.
- [ ] Moving the caret, selecting text, IME composition, editing, hiding, opening Settings or disabling autocomplete makes an old prediction unavailable. Delayed replies cannot revive it. Preview wrapping/layout changes alone do not invalidate an otherwise current prediction or create a request loop.

**Verification:**
- [ ] `cargo test --bin orbit english_assistance::tests`, `cargo test --bin orbit search_edit::tests` and `cargo test --bin orbit shortcuts::tests` pass. New regression coverage must include production reply-to-preview conversion, not just manually injected preview properties.
- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo build --release` pass.
- [ ] Follow the real isolated Settings → English assistance → Back → Search anything path; type the examples, press physical Ctrl+E and Tab, click Accept word, and observe accepted text plus Undo/Redo. Check a wrapped query, non-ASCII context and a safe Prefix body; restart the normal daemon hidden afterward.

**Dependencies:** Task 1.

**Files likely touched:**
- `src/english-assistance.rs`
- `src/shortcuts.rs`
- `ui/launcher.slint`
- `doc/english-assistance.md`

**Estimated scope:** Medium: 3–4 files. Reuse `src/search-edit.rs` unchanged unless a reproduced native-editing defect requires a fix there; if needed, this task remains within five files.

## Checkpoint: After Tasks 1–2

- [ ] New prompts and the selected model have recorded live sample results, including failures and latency.
- [ ] Source-related validation rejects wrong-word output and the preview matches accepted text.
- [ ] Physical shortcuts, mouse acceptance, Prefix priority and native Undo/Redo pass in isolation.
- [ ] Release binary is built, affected documentation matches behavior, and normal Orbit runs hidden.

## Task 3: Make suggestions and grammar feedback clearly actionable

**Description:** Finish the complete suggestions and grammar paths using the revised prompt contracts. Group the autocomplete preview, Accept word and its shortcut hint under Autocomplete instead of the last operation's status. Preserve useful independent results when another operation is empty or fails, and keep each edit tied to its current source snapshot.

**Acceptance criteria:**
- [ ] Suggestions contain up to three distinct actual rewrites, with the smallest useful correction first, rather than advice such as `Add more clarity`. Fixed cases preserve the subject, Ahmed, 3 reports, Friday and question intent. Clicking a suggestion replaces only the query, restores focus, never submits and supports one-step Undo/Redo. Empty/unchanged results do not create filler rows.
- [ ] `She go to school every day.` produces the intended `go` → `goes` correction with a present-tense explanation. `I want to learn` has no visible issue; the currently edited `concen` is not presented as a grammar mistake. Earlier completed mistakes remain checkable. Grammar clicks apply only the exact validated phrase, preserve Unicode and Prefix state, support Undo, and trigger a fresh check.
- [ ] Autocomplete's Accept word and Ctrl+E hint are visibly associated with Autocomplete. Grammar completion/error cannot change that action or overwrite its usable prediction. Empty/malformed/error results never modify the query, and Retry uses current text while successful sibling results remain usable.

**Verification:**
- [ ] `cargo test --bin orbit english_assistance::tests` and `cargo test --bin orbit search_edit::tests` pass with regressions for tail protection, exact/ambiguous/overlapping spans, unchanged issues, partial failure and stale results.
- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo build --release` pass.
- [ ] In the rebuilt isolated launcher, enable all three features from Settings, type the fixed examples, observe all sections, accept autocomplete while grammar is visible, click a rewrite and a grammar correction, and verify resulting text plus Undo. Exercise the current-text Retry/error path; restart the normal daemon hidden afterward.

**Dependencies:** Tasks 1 and 2.

**Files likely touched:**
- `src/english-assistance.rs`
- `ui/launcher.slint`
- `doc/english-assistance.md`

**Estimated scope:** Medium: 3 files.

## Task 4: Verify the complete isolated user path and apply the saved selection

**Description:** Run the final release through the real entry path on an isolated display/profile, including persistence, failures and cancellation. Capture concise evidence, update behavior/structure documentation, then apply the requested model selection to the normal profile through the existing validated atomic save path. Restore the normal daemon without opening the user's launcher.

**Acceptance criteria:**
- [ ] Isolated end-to-end checks visibly confirm current-word and next-word completion, Ctrl+E/Tab/mouse parity, suggestion replacement, phrase correction and Undo/Redo. Rapid typing, caret/selection changes, Prefix priority, feature toggles, hide/reopen, a wrapped query and Arabic/English/mixed text preserve valid state and never apply stale output.
- [ ] Missing model, unavailable loopback endpoint, malformed output and timeout leave search usable with actionable Retry. Model/prompts persist across restart; built-in migration and custom-prompt preservation are verified. Record cold/warm generation latency and distinguish it from launcher show timing; report performance budgets only when their actual measurement passes.
- [ ] Normal settings use `qwen3.5:2b` and the approved built-in prompt updates without changing unrelated data, custom prompts, independent AI settings or feature switches. Isolated processes are stopped, the final release daemon runs hidden, documentation describes actual behavior, and rollback data remains available.

**Verification:**
- [ ] `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` and `cargo build --release` pass. Do not weaken or skip checks to accommodate unrelated work; report any baseline failure precisely.
- [ ] Use an isolated headless/nested compositor, test profile and runtime; verify input/capture endpoints before automation. Follow Settings → Search bar → English assistance → Back → Search anything through each changed control, and observe resulting content/state. Do not use global input on the active desktop.
- [ ] Record evidence under `target/english-assistance/`, update measured limits in the docs, review the task-relevant diff for preserved user work, and confirm normal daemon restart without toggling it.

**Dependencies:** Tasks 1–3.

**Files likely touched:**
- `doc/english-assistance.md`
- `doc/spec.md`
- `doc/structure.md`
- `tests/english-assistance-ui.py` only if a small isolated runner is needed for reproducible acceptance checks.

**Estimated scope:** Medium: 3–4 files, plus the existing user configuration through Orbit's save path. Do not commit user settings, screenshots or model-response artifacts.

## Checkpoint: Complete

- [ ] All four tasks meet their acceptance criteria with recorded evidence.
- [ ] No unrelated changes, new dependencies, cloud calls, second model, automatic rewrites or extra shortcuts were introduced.
- [ ] Final checks pass, source matches documentation, and the normal release daemon is running hidden.
- [ ] Report the model choice, concrete fixes, verification and material latency/quality limits in a few lines.
