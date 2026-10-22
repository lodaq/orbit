# Local English assistance for Orbit

Implemented and verified in stage order, 2026-10-22. Choose an installed local Ollama generation model (existing settings retain `qwen3:4b`); keep the selected model loaded while any assistance feature is enabled. Cloud support and automatic replacement are deferred.

## Settings and shared behavior

Settings → Search bar → English assistance contains independent Suggestions, Autocomplete and Grammar feedback checkboxes, added as their stages become available. Existing configurations default to disabled. Refresh models lists installed local models with completion capability; embedding and remote cloud models are excluded. Choose one and click Use model. Switching releases the old model and warms the new selection. Three editable prompt fields show the exact system instructions for Suggestions, Autocomplete and Grammar feedback; Save prompts persists them. The query is sent separately as the user message, and JSON schemas/range validation remain enforced. The endpoint is a loopback HTTP address (default `http://127.0.0.1:11434`); remote/cloud destinations are not accepted. Test connection uses a fixed sample, never the current search text.

A dedicated background worker owns Ollama requests. Only the newest pending text is retained. Every result has a text snapshot and generation; edits, selection/caret changes, hiding Orbit, disabling features and opening Settings invalidate stale results. Model loading and requests never block typing, rendering or search. Enabled features warm the model asynchronously. Disabling all features or normal termination requests model unload. SIGINT/SIGTERM/SIGHUP are handled so normal termination can release the model; SIGKILL cannot run cleanup.

Wait 150 ms after partial-word edits, or schedule immediately after a word boundary. These are scheduling delays, not model-response guarantees. Process only ordinary English query text. Keep Prefix Tokens/badges separate from query text. Ordinary URL/TickTick Prefix query bodies are supported; skip Shell Action bodies, URLs, paths, command-like text and TickTick selectors when their syntax cannot be protected. Do not send app inventory, search results, file contents or task data to Ollama. Do not log query text or model responses. Keep suggestions in memory only.

Request structured output and validate it before showing it. Limit text to 4,000 UTF-8 bytes and 200 words, alternatives to three, and model responses to a bounded size/token budget. Timeout and malformed results leave the original query usable and offer Retry. Display measured response time. The model may produce imperfect or meaning-changing wording; choosing a suggestion is always explicit.

## Stage 1 — Suggestions

After two words, show up to three distinct improved natural versions of the sentence below Search anything, above regular search results. Do not force tone labels or fill the list with duplicate/unchanged text. The user's sentence remains untouched until a suggestion is clicked. Choosing one replaces the sentence, restores field focus and never submits the search. One Undo restores the original sentence; Redo reapplies the chosen version.

Preserve native text editing and undo history. Slint stores replacement as deletion plus insertion; the native editing bridge groups those records for assistance replacements, while retaining ordinary typing history. Escape dismisses assistance before clearing search. Editing starts a new suggestion cycle.

## Stage 2 — Autocomplete

Show one predicted next word in muted text after the caret at the end of the query. Do not request a continuation immediately after sentence-ending punctuation. Ctrl+E accepts exactly that word; Accept word provides the same action with the mouse. Tab also accepts the visible next word; an exact Prefix Token takes priority. Add configurable Accept next word through the existing shortcut settings, without taking an already assigned chord. Typing, moving the caret, selecting text or changing state invalidates the prediction. Do not insert automatically.

## Stage 3 — Grammar feedback

Check English text automatically. Show a labeled grammar section below sentence suggestions. Each issue shows an original phrase, clickable replacement and short explanation in simple English. Clicking changes only that phrase, supports Undo, and rechecks the updated sentence. Correct text may produce no issues. Validate every range against the source snapshot and UTF-8 boundaries; reject ambiguous matches or protected syntax.

## Model measurements

On 2026-10-22, the installed Qwen model advertised thinking enabled with only `true` as its supported level. A normal structured request exhausted 512 tokens on reasoning: 31.6 s cold and 27.6 s warm with no answer. A direct request using `think: false` together with a constrained JSON schema empirically returned valid short answers, despite that capability advertisement. Without the JSON constraint, reasoning leaked into content and took 54.4 s. Recheck this combination on model/Ollama upgrades.

Using the constrained direct-output request, context 2048 and a 512-token budget, one cold suggestion request took 7.01 s (3.85 s loading), followed by warm requests of 2.52 s and 2.42 s. These are sample timings, not an instant-response promise or broad quality benchmark. Ollama reported 2.8 GB allocated, 33% CPU / 67% GPU, on the user's i7-9850H and 4 GB Quadro T2000. This model resource cost is separate from Orbit RSS. Reproducible sample results are in `target/english-assistance/benchmark.json`.

## Verification results

All three stages were built in release mode, restarted and exercised through Settings → Search bar → English assistance and the real search field before moving to the next stage. Sentence replacement, next-word acceptance, individual grammar corrections and native Undo were observed in the running app. Tab still activates Prefix Tokens and otherwise accepts a visible prediction. Correct input returned no grammar issues. Grammar instructions explicitly exclude incompleteness, missing information, optional objects and stylistic completion of a short query. A model-generated unchanged correction is discarded; the valid short query “I want to learn” was checked with that behavior. A deliberately unavailable loopback endpoint produced actionable feedback with the source text and search results intact. Disabling every checkbox unloaded the model. Settings persisted across restart, and normal SIGTERM shutdown unloaded Qwen in the observed check (about 0.05 s).

`cargo fmt --check`, Clippy with warnings denied and the complete binary unit suite passed: 71 passed, 6 opt-in live-account tests ignored. Regression checks include duplicate suggestions, one-word validation, overlapping/ambiguous grammar spans, Unicode edits, native undo/redo, shortcut migration conflicts, protected Prefix query bodies, stale results and latest pending work.

Warm sample timing: next-word prediction about 1.0 s; sentence alternatives 2.4–3.0 s; a grammar correction 4.2–4.6 s, or about 0.7 s for an empty issues result. These stages run in priority order, so grammar can arrive after completion and suggestions. Qwen is not an instant-response model on this machine.

Orbit RSS was 146.41 MB with assistance disabled after Settings/search, and 147.50 MB after enabling, warming and checking the same sentence. A later hidden measurement was 147.64 MB. This is above the documented 140 MB budget even with assistance disabled; the budget is not claimed as passed. Ollama's separate 2.8 GB allocation is not part of Orbit RSS.

Five warm toggles were verified by matching background pixels in compositor screenshots. Toggle-to-confirmed-capture bounds were 126.60, 136.00, 119.58, 135.34 and 196.14 ms. Those totals include CLI startup, screenshot capture, PNG decoding and verification; they are not Wayland presentation-feedback timings and cannot establish whether the 50 ms first-frame budget passes. Raw sample artifacts are under `target/english-assistance/`.

## Acceptance checklist

- Real entry path: Settings → Search bar → English assistance → enable feature → Back → type in Search anything → use the changed control.
- Suggestions coexist with search results; selecting never submits; Undo/Redo restore text.
- Rapid edits, cancellation, feature changes and hiding Orbit cannot apply stale output.
- Protected syntax and Unicode text remain intact. Missing Ollama/model, offline endpoint, timeout and malformed JSON leave search usable.
- Ctrl+E accepts one word and remains configurable without conflicts; Tab still activates Prefix Tokens and otherwise accepts a visible prediction.
- Grammar corrections affect only the intended phrase and trigger a fresh check.
- Settings survive restart; old configurations start disabled. No model work blocks showing Orbit.
- Run formatting, Clippy and relevant regression checks, build release, restart Orbit and verify the real launcher before proceeding to the next stage.

## References

- [Ollama chat API](https://docs.ollama.com/api/chat)
- [Ollama structured outputs](https://docs.ollama.com/capabilities/structured-outputs)

## Model choice and prompt editing (2026-10-22)

Speed-first recommendation: `qwen2.5:1.5b`. A small comparison using Orbit's actual default prompts, context 2048, temperature 0.2, structured JSON and thinking disabled measured the following on this machine. Each cell is one warm request, not a percentile or accuracy benchmark. Loading is measured separately and benefits from the OS file cache.

| Model | Load | Next word | Suggestions | Grammar |
| --- | --- | --- | --- | --- |
| qwen2.5:1.5b | 1.66 s | 0.31 s | 0.38 s | 0.98 s |
| qwen2.5-coder:3b | 2.88 s | 0.57 s | 1.45 s | 1.86 s |
| gemma3:4b | 5.99 s | 1.20 s | 2.30 s | 6.04 s |
| qwen3:4b | 2.98 s | 1.18 s | 2.90 s | 4.76 s |

All four corrected “She go to school every day.” The smallest Qwen produced one acceptable rephrasing; Qwen3 produced three closer alternatives to “I want improve my English.” The coder model repeated an existing word in its completion sample. These examples do not establish general accuracy. Ollama reported 1.1 GB for Qwen2.5 1.5B, fully on the GPU at context 2048; this is separate from Orbit memory. Larger installed models and the custom 32k alias were not benchmarked. Raw replies and timings: `target/english-assistance/model-comparison.json`. Ollama lists installed models through [its tags API](https://docs.ollama.com/api/tags); the tested [Qwen2.5 1.5B model](https://ollama.com/library/qwen2.5:1.5b) matches the user's installed digest.

Ctrl+E inserts through Slint's native composition commit so a held Ctrl modifier cannot suppress insertion. Native Undo remains available. Tab uses the same acceptance callback after checking Prefix activation. The search-engine select outside the left of Search anything opens the engine list and immediately saves the selected default without submitting a query.

Follow-up verification: the rebuilt launcher listed local models, switched to Qwen2.5 1.5B and warmed it, saved and restored an edited Suggestions prompt, and preserved model/prompts/default engine across restart. Physical Ctrl+E and Tab accepted predictions; native Undo restored the query. The engine menu opened on one click and saved a choice without launching. Original Qwen3 and Perplexity selections were restored after testing. Formatting and Clippy passed; 71 unit tests passed, with 6 opt-in account tests ignored.
