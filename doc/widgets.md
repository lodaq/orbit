# Widgets

Widgets are Orbit-built tools on Home Screen pages. They provide useful information or actions at a glance. Users choose and arrange widgets from Orbit's Widget Gallery; they do not need to code or install widget code.

## Supported widgets

TickTick is the only supported widget integration. It provides two separate widgets and requires an internet connection:

### TickTick Tasks

Shows task lists and calendar views, including Inbox, Today and Next 7 Days. Users can refresh tasks, add and complete tasks, edit details, move tasks between lists and delete tasks.

### TickTick Focus Task

Shows the first unfinished task from Today, including overdue tasks. Tasks are ordered by priority and date. The widget shows task details; clicking the title opens the task editor, and its checkbox completes the task.

Both widgets use the user's TickTick connection. Keep credentials in the operating system credential store. Run requests in the background and keep Orbit responsive during loading or service errors. Apply changes immediately where possible, then reconcile with TickTick or restore the previous view if a change fails.

## Proposed widgets

### Quick Note

Future widget proposal: provide one editable scratchpad on a Home Screen page for short thoughts that are not yet tasks. Save text locally as the user types and restore it across Orbit restarts. Keep editing available offline, preserve line breaks, and let the user clear the note with confirmation. If multiple Quick Note instances are placed, each keeps its own text.

### Random Quote

Future widget proposal: show a quote chosen from a bundled local collection. Keep the displayed quote stable until the user refreshes it, and save the choice so reopening Orbit does not change it unexpectedly.

### Random Fact

Future widget proposal: show a fact chosen from a bundled local collection. Keep the displayed fact stable until the user refreshes it, and save the choice across restarts.

### Motivation

Future widget proposal: show a short motivational message chosen from a bundled local collection. Change it automatically once per local day, keep it stable on repeated Orbit opens that day, and save the choice across restarts. Let the user refresh it manually.

### Pomodoro Timer

Future widget proposal: alternate focused work sessions with short breaks, with a longer break after a configurable number of work sessions. Start with 25 minutes of work, 5 minutes for a short break and 15 minutes for a long break; let the user change these durations. Show the current phase, time remaining and completed work sessions, with Start, Pause, Resume, Skip and Reset controls. Save the phase and deadline so hiding or reopening Orbit does not restart the timer. Notify the user when a phase ends, including while Orbit is hidden, and let the user choose whether the next phase starts automatically. Work offline and keep timer updates from slowing the Launcher.

### Vocabulary Builder

Not implemented or part of the current approved widget scope. Build note: each time Orbit opens, show the next English word from the word list, with its Arabic translation, a dictionary definition and usage examples with Arabic translations. Include saved custom words in the same rotation as dictionary words. Do not repeat a word until the full list is exhausted; then start over. Let users remove custom words from the rotation. Let users mark a word as learned; record it in Achievements and exclude it from future rotations. Let users undo that choice to return the word to the rotation and remove its learned entry from both today's achievements and the full history. Support English and Arabic only. Use dictionary data bundled with Orbit or downloaded in advance. Later, let users paste an English word to look it up locally first. If it is missing, search online once and save the result locally for later offline use. If the online lookup fails, keep the pasted word and show a Retry option. Audio pronunciation is also a later enhancement.

### Affirmation

Future widget proposal: show one positive affirmation per day, selected from a shuffled list. Keep it the same across repeated Orbit opens that day. Users can add their own affirmations, and Orbit includes a starter list. Let the user mark the affirmation as read or done; then remove it from view, show a brief thank-you, and record the completion as an achievement.

- I can take this one step at a time.
- I am learning at my own pace.
- Small progress still counts.
- I deserve patience and kindness.
- I can ask for help when I need it.

### Achievements

Future widget proposal: show only today's completed tasks, affirmations marked as read or done, and words learned in Vocabulary Builder. Include tasks completed directly in TickTick as well as in Orbit after synchronization, under the date they were completed rather than the date Orbit received them. Keep the complete achievement history saved and accessible through a History button, grouped by date; a new day changes the default view but does not erase earlier achievements. Later, allow users to sync achievements to Google Calendar once at the end of each day as one entry listing all achievements for that date; keep the local history available offline. Save each day's submission status locally and show it in History. If sending fails, save the failed status and retry later until Google Calendar confirms success. Mark the day as submitted only after confirmation, and do not resend successful entries.

### Quran Player

Future widget proposal based on the [Quran Tab](https://chromewebstore.google.com/detail/quran-tab/afaihcdgkjebgabomemccdneglknjkdd) reference: show a Quran passage in Arabic with its translation or transliteration. Provide audio controls for repeat, previous, play or pause, next and playback speed, plus passage selection and saved verses. Play recitations online by default. Let users download a selected Surah for offline playback.

### Azan Reminder

Future widget proposal based on the same reference: show the next prayer, time remaining, and the day's prayer times. Do not add Azan settings or manual time entry. At the start of each local day, automatically fetch the current location and that day's prayer times online once, then save them locally. If the fetch fails, show the last successful times with their saved date and a clear warning, retry later, and withhold prayer countdowns and reminders until today's times are available.

### Habit Tracker

Future widget proposal: let users add multiple habits. Each habit has its own icon, name, description, reminder schedule, tracking method, settings and history. When a habit is completed, show a brief thank-you and add that completion to Achievements.

Other widget ideas are proposals until they are documented and approved. Do not present proposed widgets as available.

## Shared behavior

- Prefer offline operation. Use local data instead of online APIs when practical. Widgets that depend on an online service, such as TickTick, are exceptions and must remain usable when the service is unavailable.
- A widget is a first-class page item with a stable instance ID and a rectangular grid footprint. Widgets cannot live in a folder or dock, overlap another item, or span pages.
- Add widgets from the Widget Gallery and place them in free space, or use automatic placement. Escape cancels placement. Show feedback when a placement is invalid.
- Move a widget by its header, handle or a non-interactive part of its surface. Resize it with its resize control; dimensions snap to supported whole-cell spans. Interactive content keeps its own clicks and scrolling and must not start a drag.
- Refresh and other actions affect the selected widget instance unless explicitly account-wide. Content updates do not change the widget's position or size.
- A widget's context menu offers supported actions such as Refresh, Resize, Move to Page and Remove widget. Confirm removal. Layout lock prevents adding, moving, resizing and removing widgets, but allows widget actions and editing existing content.
- Preserve instance IDs, configuration, selected list and user choices across saves, restarts, page movement and layout changes. Keep a widget and its saved state if a service is unavailable; show a retryable error.

## Adding a proposal

For each widget, record its status, purpose, content, its own settings, required local data or dictionaries, any required account or online service, placement and size, user actions, refresh behavior, failure recovery and persistence needs. Keep proposed widgets separate from supported widgets. Keep authentication secure, run network and other slow work off the UI thread, and never execute arbitrary downloaded code. Check the approved UX spec and glossary before changing product behavior or UI wording.
