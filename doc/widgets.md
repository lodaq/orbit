# Widgets

Widgets are Orbit-built tools on Home Screen pages. They provide useful information or actions at a glance. Users choose and arrange widgets from Orbit's Widget Gallery; they do not need to code or install widget code.

## Supported widgets

TickTick is the online widget integration. It provides two separate widgets and requires an internet connection:

### TickTick Tasks

Shows task lists and calendar views, including Inbox, Today and Next 7 Days. Users can refresh tasks, add and complete tasks, edit details, move tasks between lists and delete tasks.

### TickTick Focus Task

Shows the first unfinished task from Today, including overdue tasks. Tasks are ordered by priority and date. The widget shows task details; clicking the title opens the task editor, and its checkbox completes the task.

Both widgets use the user's TickTick connection. Keep credentials in the operating system credential store. Run requests in the background and keep Orbit responsive during loading or service errors. Apply changes immediately where possible, then reconcile with TickTick or restore the previous view if a change fails.

## Local widgets

Random Quote, Random Fact, Motivation, Affirmation, Vocabulary Builder, Achievements, Quran Player, Azan Reminder and Habit Tracker now have local Home Screen cards in Add item → Widgets. Add them with Auto or Place. Settings → Data manages their saved collections. Each collection has its own table in the single `widgets.db` database: `quotes`, `facts`, `motivation`, `affirmations`, `vocabulary`, `quran_player`, `azan_reminder`, `habits` and `achievements`. `widget_rotation` stores per-instance selections and shuffled rotation progress.

Achievements has `id`, `date` and `finished_ids`; `finished_ids` is a JSON array of references such as `habits:2`, `affirmations:3`, `vocabulary:1` and `ticktick:<task-id>`. References include the source type because table-local IDs may overlap. Habit checkboxes, Done, Learn and Undo update the matching daily row. Vocabulary learning persists and excludes learned words from later selections. TickTick completions record their IDs and completion dates, including completions received after sync. Schema 6 also retains the task title in `achievement_task_titles` so Achievements displays its name offline and after restart. Sync backfills titles for existing references; blank titles do not replace a known name. Task notes and descriptions remain in TickTick. The Home Screen shows today; Settings → Data shows history.

The database initializes on a worker at daemon startup. Original English and Arabic starter text, standard facts, vocabulary examples, two unfinished habits and Quran 1:1 are inserted once during migration. Today's achievement row starts with an empty `finished_ids`; no activity is invented. User edits and IDs survive migration. Deleting starter entries is preserved across restarts. Quran text references [Quran.com](https://quran.com/1/1). The prayer-time starter is an explicitly labelled Cairo example for 2026-10-25 from [Aladhan, Egyptian calculation method](https://api.aladhan.com/v1/timingsByCity/25-10-2026?city=Cairo&country=Egypt&method=5); check city and date before relying on it. Stale schedules show their date and a warning.

Current limits: Fetch review/import is not implemented. Quran audio can be chosen locally in Data, copied into Orbit's `audio/` folder and opened with Play in the system audio application; embedded pause/repeat/speed controls are still planned. Azan and Habit desktop reminders are still planned. Prayer schedules can currently be edited locally in Data; this supersedes the earlier no-manual-entry proposal. The target behavior below records the remaining feature details.




### Random Quote

Target behavior: show a quote chosen from a bundled local collection. Change the quote automatically once per local day and keep it stable on repeated Orbit opens that day. Let the user refresh it manually for another quote, and save the current choice across restarts. Use a shuffled collection without repeating an entry until every entry has been shown, counting both daily changes and manual refreshes; then reshuffle and begin again. Save rotation progress across restarts. Mix English and Arabic entries in the same rotation. Show each entry in its own language, without a paired translation, and use readable text direction for that entry.

### Random Fact

Target behavior: show a fact chosen from a bundled local collection. Change the fact automatically once per local day and keep it stable on repeated Orbit opens that day. Let the user refresh it manually for another fact, and save the current choice across restarts. Use a shuffled collection without repeating an entry until every entry has been shown, counting both daily changes and manual refreshes; then reshuffle and begin again. Save rotation progress across restarts. Mix English and Arabic entries in the same rotation. Show each entry in its own language, without a paired translation, and use readable text direction for that entry.

### Motivation

Target behavior: show a short motivational message chosen from a bundled local collection. Change it automatically once per local day, keep it stable on repeated Orbit opens that day, and save the choice across restarts. Let the user refresh it manually. Use a shuffled collection without repeating an entry until every entry has been shown, counting both daily changes and manual refreshes; then reshuffle and begin again. Save rotation progress across restarts. Mix English and Arabic entries in the same rotation. Show each entry in its own language, without a paired translation, and use readable text direction for that entry.

### Vocabulary Builder

Target behavior: each time Orbit opens, show the next English word from the word list, with its Arabic translation, a dictionary definition and usage examples with Arabic translations. Include saved custom words in the same rotation as dictionary words. Do not repeat a word until the full list is exhausted; then start over. Let users remove custom words from the rotation. Let users mark a word as learned; record it in Achievements and exclude it from future rotations. Let users undo that choice to return the word to the rotation and remove its learned entry from both today's achievements and the full history. Support English and Arabic only. Use dictionary data bundled with Orbit or downloaded in advance. Let users paste an English word to look it up locally first. If it is missing, offer Fetch; search online only when the user clicks it, show the result for review, and save it only if the user accepts it. If fetching fails, keep the pasted word and let the user click Fetch again. Audio pronunciation is also a later enhancement.

### Affirmation

Target behavior: show one positive affirmation per day, selected from a shuffled list. Keep it the same across repeated Orbit opens that day. Users can add their own affirmations, and Orbit includes a starter list. Let the user mark the affirmation as read or done; then remove it from view, show a brief thank-you, and record the completion as an achievement.

- I can take this one step at a time.
- I am learning at my own pace.
- Small progress still counts.
- I deserve patience and kindness.
- I can ask for help when I need it.

### Achievements

Target behavior: show only today's completed tasks, affirmations marked as read or done, and words learned in Vocabulary Builder. Include tasks completed directly in TickTick as well as in Orbit after synchronization, under the date they were completed rather than the date Orbit received them. Continue recording achievements while Orbit is running even when no Achievements widget is placed on a page. The widget displays today's achievements only. Keep the complete achievement history in Settings → Data, grouped by date and available even when no Achievements widget is placed. Removing the widget does not erase history; a new day changes the widget's view but does not erase earlier achievements. Later, allow users to sync achievements to Google Calendar once at the end of each day as one entry listing all achievements for that date; keep the local history available offline. Save each day's submission status locally and show it in Settings → Data. If sending fails, save the failed status and retry later until Google Calendar confirms success. Mark the day as submitted only after confirmation, and do not resend successful entries.

### Quran Player

Target behavior based on the [Quran Tab](https://chromewebstore.google.com/detail/quran-tab/afaihcdgkjebgabomemccdneglknjkdd) reference: show a Quran passage in Arabic with its translation or transliteration. Provide audio controls for repeat, previous, play or pause, next and playback speed, plus passage selection and saved verses. Play recitations from downloaded local audio only. Let users click Fetch to review available downloads for a selected Surah and choose which files to save locally. Playback uses accepted local files only and never starts a download.

### Azan Reminder

Target behavior based on the same reference: show the next prayer, time remaining, and the day's prayer times. Manage the saved schedule and its date in Settings → Data. Clicking Fetch retrieves current prayer-time data for review. Save a schedule locally only after the user accepts it. A new day does not fetch automatically. If today's times are unavailable or fetching fails, show the last accepted times with their saved date and a clear warning, and withhold prayer countdowns and reminders until today's times are available. The user clicks Fetch to try again.

### Habit Tracker

Target behavior: let users add multiple habits. Each habit has its own icon, name, description, reminder schedule, settings and history. Send habit reminders as desktop notifications at the user-selected times, including while Orbit is hidden. The first version uses a daily checkbox: each habit can be completed once per local day. Count and duration tracking are later additions. When a habit is completed, show a brief thank-you and add that completion to Achievements. Let users undo a completion; undo removes the matching completion from the habit history and Achievements, including the full history in Settings → Data. Deleting a habit stops its future tracking and reminders but preserves its past achievements in Settings → Data.

Other widget ideas are proposals until they are documented and approved. Do not present proposed widgets as available.

## Data sources

### Offline-first approach

Make widgets usable offline as much as possible. Display saved local content and keep local interactions available without internet access. The user explicitly initiates external updates with Fetch; there are no automatic API fetches, scheduled resource updates or automatic network retries. Fetch presents retrieved entries for review and does not modify the local store until the user accepts selected entries. Refreshing the displayed item selects from locally saved content. Daily rotations also use saved content only. If a manual fetch fails, retain existing content and let the user fetch again later. TickTick widgets remain excluded as described below.

Keep all Orbit data together in the existing Orbit data folder. Widget database files and downloaded resources live there alongside Orbit configuration and icons; one folder may contain multiple files.

Use one SQLite database for data managed by Settings → Data, including the non-TickTick widgets' saved content and progress. Keep downloaded Quran audio as files in a subfolder of the same Orbit data folder, with database records pointing to those files. Keep the existing Orbit JSON configuration files and icons in this same folder. Do not copy TickTick task data into the database.

TickTick Tasks and TickTick Focus Task are excluded from this local content storage model and the manual Fetch rule. Do not add persistent local copies of their task data; retain the existing TickTick service integration and credential storage. Existing saved widget placement and selected-list settings are unaffected.

For all other widgets, display only data already stored locally in Orbit. Clicking Fetch retrieves remote candidates for review. The user selects which entries to accept; Fetch never automatically adds or replaces local data. Opening Orbit, opening a widget, changing the day or rotating content never initiates a remote fetch. There are two ways to populate the local store:

- **Local data:** content stored in Orbit. Users can add, edit and delete entries through Settings → Data.
- **Fetched candidates:** content retrieved from an external service through an API. Candidates are not available to widgets unless the user accepts them into local storage. Each source defines its service connection and failure recovery.

Settings → Data is the single place to manage Orbit data. It provides editing for the locally stored data of every widget covered by this storage model, including user-added, bundled and API-imported content. Users can add, edit and delete saved entries offline. Editing fetched content changes Orbit's local copy and is never sent back to the remote source. Preserve its stable ID and do not overwrite user edits on later Fetch actions. TickTick remains excluded; full achievement history remains in Settings → Data.

Widgets use one local content collection that can contain user-created, bundled and accepted fetched entries. Downloading resources is separate from displaying or rotating widget content. Users retrieve remote resources only by clicking that category's Fetch action in Settings → Data; there is no scheduled fetching or automatic network retry. Settings → Data presents fetched candidates in a review list with an individual Add choice for each candidate. Show a short preview for every data type, with an option to expand a candidate and view its full content before adding it. Start with no candidates selected. Adding a candidate saves it to the local collection; leaving the review list discards unadded candidates, which can be retrieved again by Fetch. Daily changes and manual Refresh choose from saved local content. A failed Fetch preserves the previously saved collection.

Give every saved entry a stable Orbit ID. Widget instances read from the local collection. Show fetched candidates as returned, including duplicates. The user decides which candidates to add. Save every selected candidate, even when its content duplicates an existing entry; do not deduplicate, hide, or block it. Fetch does not change the database by itself. Do not add a separate source-tracking or background-sync system. These storage requirements describe proposed behavior for widgets other than TickTick.

Deleting a fetched entry removes it from the local collection. A later Fetch may present it again as a candidate if the API still returns it; it is saved again only if the user accepts it.

## Shared behavior

- Settings → Data is one place to control Orbit data. Organize it by category, with an icon, storage size, saved contents and controls for each category. Include app configuration and icons, widget entries and progress, downloaded Quran audio, habits, reminders and Achievements. Show total storage use and separate database size from downloaded-file size; count shared entries once. Let users browse, search, add, edit and delete eligible saved data, including custom Motivation content and Quran files. Provide Fetch controls to retrieve external candidates for review and allow users to accept selected data into local storage. Opening Settings → Data works offline and never fetches by itself. Exclude TickTick task data.
- Widget text and user-authored content can be English or Arabic, including mixed-language text, with readable text direction. Users can add custom content to content collections such as Random Quote, Random Fact, Motivation and Affirmation; custom entries participate in the widget's normal rotation alongside bundled entries. Custom entries belong to a shared collection for that widget type, so adding an entry once makes it available to every instance of the same type. Settings → Data lets users add, edit and delete entries in these shared collections, including when no instance of the widget is placed on a page.
- Except for the TickTick widgets, read displayed content from local storage. Remote candidate retrieval runs on a worker only after the user clicks Fetch; accepted content is saved locally and remains available offline. TickTick retains its existing service-specific request and failure rules.
- A widget is a first-class page item with a stable instance ID and a rectangular grid footprint. Widgets cannot live in a folder or dock, overlap another item, or span pages.
- Add widgets from the Widget Gallery and place them in free space, or use automatic placement. Escape cancels placement. Show feedback when a placement is invalid.
- Move a widget by its header, handle or a non-interactive part of its surface. Resize it with its resize control; dimensions snap to supported whole-cell spans. Interactive content keeps its own clicks and scrolling and must not start a drag.
- Refresh and other actions affect the selected widget instance unless explicitly account-wide. Content updates do not change the widget's position or size.
- A widget's context menu offers supported actions such as Refresh, Resize, Move to Page and Remove widget. Confirm removal. Layout lock prevents adding, moving, resizing and removing widgets, but allows widget actions and editing existing content.
- Preserve instance IDs, configuration, selected list and user choices across saves, restarts, page movement and layout changes. Keep a widget and its saved state if a service is unavailable; show a retryable error.

## Adding a proposal

For each widget, record its status, purpose, content, its own settings, required local data or dictionaries, any required account or online service, placement and size, user actions, refresh behavior, failure recovery and persistence needs. Keep proposed widgets separate from supported widgets. Keep authentication secure, run network and other slow work off the UI thread, and never execute arbitrary downloaded code. Check the approved UX spec and glossary before changing product behavior or UI wording.

## To add later

- daily english word
- daily english idom
- daily english sentence
- daily engilsh quote
- daily arabic word

- daily islamic question (flip card)
- daily hadith
- daily dua
- Daily sunnah
- daily ayah with Interpretation
- daily quran with Interpretation
- Daily azkar (night, mornning, wake up, sleep) with reminders and notifications

- daily life hack
- daily tip
- daily quote
- daily affirmations

## Widget presentation

All eleven widgets share Orbit's quiet header, widget surface and text roles. Local content uses a reading hierarchy appropriate to its type: quotes and Quran passages have larger text, vocabulary separates the first line from its definition/example, and Azan separates the next prayer from the schedule. This changes display only; saved content remains intact. Habit rows expose their completion state with grouped surfaces. Local actions stay in a fixed footer while content scrolls. Header dragging, resizing, local completion/undo and TickTick entry points retain their behavior. Gallery previews show content specific to each widget.

## Morning Remembrances (Adhkar)

Approved on 2026-10-27 from the user's supplied Arabic content and visual references. Add item → Widgets → Morning Remembrances offers Auto and Place, with a 3 × 5 default and 3 × 3 minimum footprint. All 25 supplied entries are bundled offline; Settings → Data → Morning Remembrances edits their title (first content line), text, source details and repetition count.

Each counter click records one repetition. Reaching the required count plays a short chime and advances to the next unfinished remembrance. Finishing every entry plays a different chime and shows the completion screen. Skip leaves an entry unfinished and cycles back to unfinished entries. Previous retains counts. The centered gold الفضل والمصدر button has a visible surface, border and expansion arrow below the repetition counter and divider; source details expand in a bounded scroll area above the fixed controls. Restart clears only this widget's current progress. Each widget saves independent daily progress in widgets.db and begins fresh on a new local day. Rapid clicks from an old entry cannot count toward its successor. Sounds play in the background through the desktop audio player.

Only remembrance and source text scroll. The repetition counter, source control and navigation stay fixed below the reading area. Short widgets place a compact counter beside the source control and navigation. Reading and source text use equal horizontal insets; vertical scrollbars sit in the right gutter outside the content width.

## Evening Remembrances (Adhkar)

Requested on 2026-10-27 with 25 supplied evening entries. Add item → Widgets → Evening Remembrances offers Auto and Place, using the same 3 × 5 default and 3 × 3 minimum footprint as Morning Remembrances. The Arabic heading is أذكار المساء. The text, repetition counts, source details and notes retain the supplied wording and order.

Evening Remembrances shares the morning widget's reading layout, smaller fixed counter, navigation, source details, completion sounds and Restart behavior. Both remembrance widgets leave 12 px beneath their controls. Its entries live in a separate editable Evening Remembrances collection in Settings → Data. Schema 4 adds this collection once while preserving morning content and progress. Progress remains independent for every widget instance and resets on a new local day.

Verify adding the widget from the gallery, evening-specific text, counting and automatic advancement, Skip/Previous, expanded source text, Restart, persistence after restart, and independent morning/evening progress. Run `cargo test`, `cargo fmt`, `cargo clippy -- -D warnings` and `cargo build --release`, then restart Orbit and follow the real Add item path in the running launcher.

## Azan location settings and reading layout

Requested 2026-10-28. Azan Reminder uses a quiet location row with a settings icon, a prominent next-prayer name and time, a remaining-time label and aligned daily prayer rows. The upcoming prayer is highlighted. The existing widget material and Inter remain in use; controls respond immediately without entrance motion.

Azan settings opens from the widget's settings icon, with City, Country and a named Time zone. Defaults are Sohag, Egypt and Africa/Cairo. Fetch times explicitly retrieves today's five prayer times from Aladhan using method 5 (Egyptian General Authority of Survey). Review the response in the sheet, then Save to accept it. Cancel, backdrop and the configured Close shortcut dismiss the sheet. Editing any location field invalidates the preview; late responses from a dismissed sheet are ignored. No automatic Fetch runs on opening or a new day.

Database schema 5 stores per-widget location settings and the accepted schedule reference. Accepted schedules remain editable under Data → Azan Reminder. Existing schedules are preserved there; the dated Cairo example is not adopted as a Sohag schedule. A changed city, country or time zone clears the association until a new schedule is accepted. Countdown calculations use the selected zone's date and seasonal clock rules. Missing, deleted or outdated schedules show an actionable state without countdowns. Desktop notifications remain deferred.

Azan verification (2026-10-28): an isolated X11 desktop exercised launcher → Azan settings → live Fetch times → review of all five Sohag times → Save. Verified that Fetch alone did not write a schedule, the accepted schedule survived restart, an invalid zone left saved data intact, Escape dismissed the sheet, and both wide and compact layouts remained readable with scrolling. The verified Sohag / Africa/Cairo schedule was applied to the normal profile, and the release daemon restarted without opening the active desktop. Captures: `target/ui-check/azan-x11/`. Release build, formatting, Clippy and tests passed (six existing opt-in tests remain ignored). API contract: [Aladhan](https://aladhan.com/prayer-times-api); named-zone rules: [Chrono-TZ](https://docs.rs/chrono-tz/0.10.4/chrono_tz/).

Completing every repetition in Morning Remembrances or Evening Remembrances records one daily Finished ID per widget in Achievements. Skipping and partial progress do not count. Restarting retains an earned completion. Previously completed widgets for today are recorded when widget data opens; history keeps the collection name after widget removal.
