# English assistance for Search anything

Status: proposal, 2026-10-22. This document plans the feature; it does not authorize implementation or change Orbit's approved build order. No application code has been implemented for it.

## Goal

Help the user write English in Search anything: complete unfinished text, find spelling and grammar problems, explain corrections, and offer alternative phrasing. Support both local models through Ollama and a cloud provider, with configuration under Settings → Search bar. Use Orbit's native Rust and Slint interface.

This is writing assistance, not a guarantee that a sentence is correct. Suggestions should preserve the user's meaning. Do not add facts, deadlines or promises to a rewrite.

## Confirmed requirements

- English spelling and grammar feedback in a small panel near the search field, with possible fixes and explanations.
- Several rephrasing options the user can choose from.
- Text completion and an option for automatic spelling correction.
- Both local Ollama and cloud service options.
- Settings live under Settings → Search bar.
- Planning and documentation only at this stage.

Everything below is a proposed implementation baseline. The final section lists decisions to confirm before implementation.

## Search field and panel

Add an English assistance control beside Search anything. It opens a compact panel anchored below the search field, constrained to the available screen space. Keep the field editable and preserve the current query when closing the panel. Use shared Theme tokens and existing controls.

The panel offers three actions:

1. **Check English** — identify spelling and grammar issues. Each card shows the original phrase, a suggested replacement, a short English explanation, and Apply and Dismiss controls. Also show a corrected sentence preview and Apply all. If no issues are found, say so without inventing corrections.
2. **Rephrase** — offer up to three distinct alternatives: Natural, Concise and Formal. Each has an Apply control. Keep short source text short and retain the original intent.
3. **Complete text** — suggest one short continuation at the end of the text. Show the proposed addition separately and insert it only when accepted. For the first version, make this an explicit action rather than an always-running prediction.

Use the selected text when a selection exists; otherwise use the editable query. Show which text is being checked. Applying a suggestion changes text only; it never searches, launches an item or submits an Action. Keep native undo available and restore focus to the field after applying. A rewrite should be reversible in one undo step.

Typing invalidates visible suggestions immediately. A result applies only to the exact query/selection snapshot used to request it. Closing the panel, hiding Orbit, changing providers or opening Settings cancels pending work and invalidates the result. Dismissed suggestions remain dismissed for that text snapshot.

### Example

Input: `He go to work yesterday.`

- Check English: `He went to work yesterday.` Explanation: use the past tense with “yesterday.”
- Natural: `He went to work yesterday.`
- Concise: omit this alternative if it cannot make the sentence meaningfully shorter.
- Formal: omit this alternative if it would only add unnecessary words.

Do not produce three alternatives merely to fill three slots.

## Search bar settings

Add **English assistance** alongside General, Prefixes and Actions. The subsection contains:

| Setting | Proposed behavior/default |
|---|---|
| Enable English assistance | Off until explicitly enabled. |
| Provider | Local (Ollama) or Cloud; Local selected initially. Switching never sends a request by itself. |
| English variety | American English initially; British English available. |
| Check while typing | Off initially. When enabled, check after a 700 ms pause; do not run rewrites or completion automatically. |
| Automatic spelling correction | Off initially; requires the staged safeguards below. Grammar and rephrasing always require Apply. |
| Local address | `http://127.0.0.1:11434` by default; editable for a user-managed Ollama server. |
| Local model | Choose from installed models using Refresh models; no download is started automatically. |
| Cloud connection | Provider, model, API key and any provider-specific endpoint setting. Exact first provider remains to be chosen. |
| Test connection | Use a fixed sample sentence, show connection/model errors beside these settings, and show response time. Do not send the current search query. |

Retain separate local and cloud settings so switching providers does not discard configuration. Store API keys in the operating system credential store, using the existing project approach. Save non-secret settings through the existing configuration mechanism; old configurations default to disabled.

Make the active provider visible in the assistance panel. A cloud model accessed through Ollama is still cloud processing; do not label it as local. For v1, the Local model selector should exclude known cloud models. A remote Ollama address is another computer, and the settings must make that destination clear.

## Correction and completion safeguards

Search anything also contains app names, files, URLs and commands. Never rewrite a Prefix Token or its badge. For an active Prefix, assistance operates on the query body only. Keep TickTick selectors such as `!today`, `#home` and list selectors intact; exclude commands or queries with syntax that cannot be protected safely. Do not automatically check URL-only, path-only, numeric or code-like input. Explicit checking may be offered for ordinary English text, but it must still preserve protected syntax.

Do not intercept Enter, Tab or arrow keys globally: Enter already launches/searches, and Tab already activates Prefixes. Suggestions are reachable by mouse and explicit keyboard focus. Escape closes the assistance panel before clearing search or hiding Orbit; keys used inside its focused controls must not also navigate search results. Ordinary search results remain available when the assistance panel is closed.

Automatic spelling correction is a later stage. A model's confidence claim is not sufficient evidence that an edit is safe. Limit eligible edits to completed plain-English words with a deterministic spelling check and an unambiguous replacement. Exclude names, URLs, paths, commands, Prefix syntax and mixed-language text. Apply after a word boundary, preserve native undo, and avoid repeating an undone correction. If those safeguards cannot be met, keep the setting unavailable and provide suggestions only. Select a spelling engine only after evaluating its dictionary size, licence and correction quality.

Always-on inline completion is also a later stage. Measure latency and keyboard interaction first. Never silently insert generated text or reuse Tab while it is needed for Prefix activation.

## Local model choice and measurement

The user already has `gemma3:4b`, `qwen2.5:1.5b` and larger models installed. Compare the first two as initial candidates; do not claim a winner without measurements. Inspect each installed model's actual capabilities rather than relying on a mutable model tag. The installed `qwen3:4b` identifier was associated with a thinking variant, so do not assume thinking can be disabled.

Observed machine: Intel Core i7-9850H, Quadro T2000 with 4 GB graphics memory. About 2.8 GB was free during inspection; this changes with other apps. File size does not determine runtime memory or latency.

Use a small, fixed English evaluation set covering spelling, grammar, natural rewrites, valid sentences needing no changes, names, short fragments, contractions, and English/Arabic mixed input. Compare preservation of meaning and explanation usefulness, not just fluency. Record cold load time, warm response time, timeouts and memory use. Report median and slow responses, not only the fastest result.

Load a model only when assistance is requested or the user explicitly enables automatic checking. Never load it during Orbit startup or launcher show. Ollama's separate process can still consume substantial system/GPU memory; report that cost separately from Orbit's resident memory. Do not add a resident language engine until its resource cost has been measured.

## Request flow and internal interface

Keep provider work off the UI thread. Reuse the existing HTTP and credential dependencies where suitable; avoid a new SDK dependency for ordinary HTTP calls.

1. Capture text, selection, protected spans, requested operation, language variety and a monotonically increasing request generation.
2. Cancel superseded work, debounce only automatic checks, and send the snapshot to the selected provider. Allow one active assistance request per field.
3. Request structured output, then validate it locally. Reject malformed output, invalid text spans, edits to protected syntax and excessive results. Never execute model output or render it as markup.
4. Return a bounded result to the UI thread only if its generation and text snapshot still match. Show provider failure with Retry; keep search and the source text usable.
5. Apply a chosen correction or rewrite through the native editing path, preserving cursor position where practical and undo history.

The shared request has an operation (Check, Rephrase or Complete), source text and English variety. Its result has either issue cards with source spans/replacements/explanations, a corrected preview, rewrite choices with style labels, or one continuation. Include request identity and the source snapshot internally. Normalize text spans to Rust UTF-8 boundaries before editing; account explicitly for Slint's cursor/selection representation and providers' differing offset conventions. Applying a single fix invalidates the previous batch unless remaining spans are recomputed against the new text.

For local Ollama, use its chat endpoint with structured output where the installed version/model supports it. Cloud adapters must normalize their provider response into the same result type. Use explicit errors if a selected model does not support a required capability. Do not silently switch providers or send local text to a cloud fallback.

Proposed limits: 4,000 characters per explicit request, at most 10 issue cards and three rewrites, and a 30-second request timeout. Show the length limit before sending; do not silently truncate. Offer Cancel during a cold load. Retry is explicit; avoid automatic retries that could duplicate cloud charges.

## Privacy and storage

Only the selected text/query body needed for the requested operation goes to the chosen provider. Do not attach search results, saved item inventory, other tasks, conversation history or file contents. Explain cloud transmission when the user enables/configures Cloud and explain that Check while typing sends eligible text automatically.

Do not log source text, suggestions or API keys. Keep results in memory for the current editing session; clear them when Orbit hides. No writing history or learning profile is proposed. Never include API keys in configuration exports; restoring a backup requires reconnecting the cloud provider. Review the chosen provider's retention, pricing and account requirements before finalizing its connection flow.

## Implementation sequence

Each stage needs separate implementation authorization; this document alone does not start them.

1. **Evaluate providers:** measure the two local model candidates on the fixed sample set; choose the first cloud provider and verify its correction/rewrite and structured-output capabilities, costs and retention policy.
2. **Connection settings:** add English assistance settings, installed model refresh, provider credentials and fixed-sample connection testing. Default disabled; keep all network work outside launcher startup/show.
3. **Check and Rephrase:** implement the common request/result contract, background worker, native panel, manual Apply/Dismiss, cancellation and undo. Make both providers available through the same user flow.
4. **Explicit completion:** add Complete text using the same request lifecycle and protected syntax rules.
5. **Optional automatic behavior:** enable debounced checking after latency evaluation. Implement automatic spelling correction only with the deterministic safeguards; evaluate inline completion separately.

Likely code responsibilities: a small English assistance module for request coordination/provider normalization; native panel/settings in the current Slint UI; existing configuration and platform credential facilities for persistence. Split files only when their responsibilities warrant it. Update the authoritative specification, UI glossary, shortcuts and file structure when the behavior is approved for implementation.

## Acceptance checks for implementation

- Both Local and Cloud can be configured, selected and tested without accidentally transmitting the current query. Disabling assistance prevents all assistance requests.
- A misspelled sentence produces useful fixes and short explanations; a correct sentence can return no issues. Rewrites preserve meaning and do not invent details.
- Apply, Apply all, Dismiss, native Undo and selected-text editing work with mouse and keyboard. Applying a suggestion never submits the query.
- Prefix activation, search results, app launching, URLs, paths and TickTick selectors keep their expected behavior. Mixed Arabic/English text does not produce invalid ranges or corrupt text.
- Fast typing, changing selection, hiding Orbit, reopening, changing model and switching provider cannot apply stale output.
- Unavailable Ollama, missing model, invalid key, rate limiting, offline cloud, timeout and malformed responses leave text intact and provide actionable feedback.
- Local mode never automatically falls back to Cloud. Keys are absent from config, logs and backups.
- Existing configs migrate to assistance disabled; provider settings survive restart. No network/model loading runs while showing Orbit.
- Measure Orbit's existing show-time/memory budgets and the separate model resource cost. Verify any automatic mode against measured latency.
- When code is eventually changed, run required formatting/Clippy/release checks, restart Orbit, and follow the real path through Settings → Search bar → English assistance and the search-field panel.

## Decisions still to confirm

- Which cloud provider and account/key should be supported first? Both local and cloud are required; a specific cloud vendor has not been selected.
- Are the proposed English-only explanations sufficient, or should Arabic explanations be offered too?
- Is an explicit Check/Rephrase/Complete first release acceptable, with automatic behavior in the later stage described above?
- Confirm proposed labels, American/British default, limits and trigger behavior before treating this draft as the implementation specification.

## References

- [Ollama chat API](https://docs.ollama.com/api/chat)
- [Ollama structured outputs](https://docs.ollama.com/capabilities/structured-outputs)
- [Grammarly public API documentation](https://developer.grammarly.com/) — current public APIs do not document the live correction/rewrite interface this design requires.
- [LanguageTool proofreading API](https://languagetool.org/http-api/) — an alternative spelling/grammar engine to evaluate if model-only checking is insufficient; it is not currently selected or a dependency.
