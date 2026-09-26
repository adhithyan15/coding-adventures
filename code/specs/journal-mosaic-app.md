# journal-mosaic-app — Journal on the standard Mosaic application ABI (J3c-1)

Issue: [#14416](https://github.com/adhithyan15/coding-adventures/issues/14416) (J3c) ·
Engine: [`journal-core.md`](journal-core.md) · ABI: [`UI38-mosaic-native-application-runtime.md`](UI38-mosaic-native-application-runtime.md)

## What it is

The Rust application behind the Mosaic `journal-app` package (J3c-2). It puts
`journal-core` behind the standard `MosaicApp` trait (`mosaic-app-runtime`),
exactly as `task-mosaic-app` does for Trestle and `engram-mosaic-app` for Engram:
it turns engine state into the package's slot values (props) and the package's
events into engine commands. Every generated native host loads it as
`libmosaic_app` through `mosaic_app_capi::export_mosaic_app!`.

J3c is split so each half is reviewable: this crate (J3c-1) has no dependency on
the UI packages; the `journal-app` package (J3c-2) composes `RecordList` and
`DraftEditor` over these props.

## The editor model

The screen is a timeline beside one editor. The editor always holds a **draft**
for a **target**:

| target | draft | Save | Delete |
| --- | --- | --- | --- |
| `New` | empty until typed | creates an entry dated **today** and makes it the target | not offered |
| `Entry(id)` | that entry's title/body, as edited | writes the edits | deletes it; target → `New` |

- **Select** a timeline row → target that entry, draft loaded from it.
- **New entry** → target `New`, empty draft.
- **Cancel** → discard edits: draft reloaded from the target (empty for `New`).
- Typing only changes the draft; nothing reaches the engine until Save. This is
  the controlled-editor contract `DraftEditor` specifies.
- Saving an entirely empty new draft is a no-op with an announcement, not an
  empty entry.

## Props (the slot contract)

| slot | type | value |
| --- | --- | --- |
| `timeline-rows` | `list<list<text>>` | `RecordList` rows `[key, heading, title, subtitle, meta, badge]`, newest day first |
| `selected-key` | `text` | the target entry's id, `""` for `New` |
| `timeline-empty` | `bool` | no entries yet (drives the empty state) |
| `draft-title` | `text` | |
| `draft-body` | `text` | |
| `delete-label` | `text` | `"Delete"` for an existing entry, `""` for `New` (so `DraftEditor` hides the button) |

Row fields:

- `heading` — the day, e.g. `Thursday, 24 September 2026`, set **only on the
  first row of each day** (`RecordList`'s flattened grouping).
- `title` — the entry's title; for an untitled entry, the opening words of its
  body; failing that, `Untitled entry`. Never empty: `RecordList` draws the title
  as the row's button, and an empty one would be an unnamed button.
- `subtitle` — the first line of the body (markdown `#` markers stripped), at
  most 100 characters, cut on character boundaries; `""` when the title already
  came from the body.
- `meta` — `""` for now: no time of day yet (see *Time zones*).
- `badge` — `★` for a starred entry.

## Events

The wire name is the raw emit name (`onX`), as every generated host sends it.

| event | payload |
| --- | --- |
| `onSelectEntry` | `{ index }` — a row of the `timeline-rows` just rendered |
| `onNewEntry` | — |
| `onTitleChange` | `{ value }` |
| `onBodyChange` | `{ value }` |
| `onSaveEntry` | — |
| `onDeleteEntry` | — |
| `onCancelEdit` | — |

An unknown event is an error naming it (UI38 §4.1), and any error leaves the app
exactly as it was, so the host can retry. Only the editor fields are saved for
rollback: every journal change goes through `journal_core::apply`, which is
already all-or-nothing, so the journal itself is never cloned per keystroke.

**Drafts are capped at the engine's own limits** (title 512 characters, body
1 MiB). A draft over the limit is refused as it is typed, not accepted and then
unsaveable, so it can never grow the state file without bound.

## Ids and time

- Entry ids are minted here, `entry-{n}`, skipping any already in use — the
  core never mints (it validates: ≤ 64 printable ASCII bytes). The counter
  uses checked arithmetic and fails rather than repeats, and `restore` refuses
  a counter above 2^53 (a tampered one at `u64::MAX` used to spin forever).
- The clock is a plain `fn() -> u64` (milliseconds since the epoch), defaulting
  to the system clock and replaced in tests.
- **Time zones.** "Today" is the user's **local** date. The host passes its UTC
  offset in `StartContext.utc_offset_minutes` (UI38 "Local time"), so an entry
  written at 20:00 in New York is filed under that evening's date, not
  tomorrow's in UTC. A host that passes no offset gets the UTC date, which was
  the behaviour before. The offset is host context, not journal state, so it is
  not in the snapshot. It is applied before the last-writable clamp, so no
  offset can date an entry past 9999-12-31. The row `meta` still shows no time
  of day, because times need the entry's own offset stored (J4).

## Persistence

`snapshot()` serialises the journal, the target and the draft (schema
`journal-mosaic-app/state`, version 1), so an unsaved draft survives a restart.
`restore` refuses a foreign schema, another version, corrupt bytes, a journal
that fails `JournalState::validate()`, an oversized draft, or an implausible id
counter; a target naming a missing entry is
repaired to `New`.

## The package (J3c-2)

`code/programs/mosaic/journal-app` exports `JournalApp`. Its `.mil` declares
exactly the slots and events above. Its layout is a `HostNavigationSplit`:

- the **pane** holds a *New entry* button (`onNewEntry`), then `EmptyState`
  when `timeline-empty` is true, else `RecordList` (`timeline-rows`,
  `selected-key`, and `onSelect` forwarded as `onSelectEntry`);
- the **detail** holds `DraftEditor`, with the draft slots and `delete-label`
  bound and its change, save, delete and cancel events forwarded.

The package's tests enforce the contract from both sides:

- the app's start props are exactly the slots;
- every declared emit is routed, and an undeclared one is rejected.

## Native hosts (J5a)

The first native host is **Qt on Linux**, in CI's "Round-trip Rust engine
through standard Qt binding" step, the same lane that launches TaskApp. It:

1. builds `journal-mosaic-app` as a cdylib (`export_mosaic_app!`);
2. emits `journal-app` with `--profile native-complete --runtime-library`, and
   requires no replaced generated files and zero degradations;
3. builds and installs the project, and checks that the installed
   `libmosaic_app.so` is the one built;
4. launches the installed `JournalApp` offscreen twice against one state file.
   Each launch must stay up for five seconds, with the runtime present and no
   missing prop or QML error; the second exercises restore;
5. runs the package's own `cargo test`.

The lane is triggered by `journal-core`, `journal-mosaic-app` and the
`journal-app` package (`mosaic_qt_runtime_ci_acceptance.py`).

**SwiftUI and Compose (J5b)** emit with the same strict binding, pin the same
empty reports, and then build: `swift build` on macOS and `gradle
compileKotlin` on Linux. The same three packages trigger their lanes. Neither
launches Journal yet; for both, a launch needs a harness like TaskApp's.
Flutter and XAML follow. The web host waits on a wasm clock:
`SystemTime::now()` panics on `wasm32-unknown-unknown`.

## The browser (J5c)

The same crate is the browser runtime. It is built for `wasm32-unknown-unknown`
and exported through `mosaic_app_wasm::export_mosaic_wasm!`, the standard
lifecycle bridge VisiCalc uses: `create`, `dispatch`, `snapshot`, `restore`
and `destroy` over `mosaic_wasm_call`. The native C ABI is unchanged.

**The clock is the host's.** `std::time::SystemTime::now()` panics on
`wasm32-unknown-unknown`, which has no clock of its own. On wasm32 the clock is
a module import, `journal.now_ms() -> f64` (milliseconds since the epoch).
The browser host passes `Date.now` through a shim that must **return**. An
exception thrown out of an import would unwind past Rust frames, so the shim
catches everything and returns NaN:

```js
{ journal: { now_ms: () => { try { return Number(Date.now()); } catch { return NaN; } } } }
```

A value that is not finite, is negative, or is later than 9999-12-31 reads as
0 (the epoch), never as a panic or a wrapped date. The upper bound matters:
journal-core writes four-digit years and reads back only 0–9999, so an entry
dated later would make the *whole* snapshot refuse to restore. The same bound
applies to the native `SystemTime` reading. The module needs the import to
instantiate.

**Event names, bare or prefixed.** Native hosts send the raw emit name
(`onSelectEntry`). The generated React component dispatches `type:
"selectEntry"`. Journal accepts both: a bare name whose first letter is
lower-case is read as `on` plus that name capitalised. This is the same
normalisation task-, VisiCalc- and SPICE-mosaic-app apply in the other
direction. An unknown event is still an error that names what was sent.

J5c-1 is this runtime and a Node test that drives it through the real
`mosaic-host.mjs` loader. The React web host that mounts `JournalApp` over it
is J5c-2.

### The web host (J5c-2)

`code/programs/mosaic/journal-app/host/web` is a Vite and React page that
mounts the generated `JournalApp` over this runtime, in the same shape as
VisiCalc's host.

- **Build:** `scripts/build-web.sh` compiles `journal-app` with the React
  backend for both themes (into `src/components/{light,dark}`, git-ignored). It
  also builds the wasm32 runtime and copies it into `public/`.
- **Boot:** the page fetches the wasm, loads it through `mosaic-host.mjs` with
  the guarded clock shim, and creates the app with the preferred colour
  scheme and the browser's UTC offset (`utcOffsetMinutes`). An implausible
  offset is left out, and the app falls back to UTC.
- **Rendering:** props are passed through with kebab-case keys turned into
  camelCase, and the component's `dispatch({type, ...payload})` goes straight
  to `host.dispatch`. The host translates nothing and owns no state. The
  runtime's announcements go to a polite live region, and a refused event's
  message to an alert.
- **Persistence:** after each dispatch the host takes `snapshot()` and stores
  it in `localStorage` under `journal-mosaic/state`. It stores the snapshot's
  bytes as text, because they are the runtime's JSON. On boot, a stored
  snapshot is restored. If the runtime refuses it, the stored value is **kept**
  under a key of its own, `journal-mosaic/state.unreadable.<time>`. It is never
  deleted, and a later refusal never overwrites an earlier one. The journal
  then starts empty, and an alert says so. If saving fails (quota, private
  mode), an alert says so and the journal keeps working in memory.
- **One writer.** When another tab changes the stored journal (a `storage`
  event), this tab stops saving and asks for a reload. Its snapshot is older,
  and writing it would silently undo the other tab's work.
- **Tests (Vitest and jsdom):** they use the real wasm to check four things:
  - the empty state renders;
  - writing and saving an entry puts it on the timeline;
  - a reload restores it;
  - an unreadable stored value is kept aside rather than lost.
- **CI:** `.github/workflows/journal-mosaic-web.yml`, modelled on
  `visicalc.yml`, runs the tests and the production build. The old TypeScript
  Journal and its `deploy-journal.yml` are untouched; retiring them is J5
  release work.

### Published at /journal/ (J5d)

The owner decided (2026-09-25) that this app **replaces** the old TypeScript
Journal on the web.

**What was there.** `deploy-journal.yml` built the old app and published it with
`actions/deploy-pages`, as a Pages *artifact*. The site is not built from
artifacts; it is built from the `gh-pages` branch, where every other app is
published into its own directory. So `/journal/` was never served: it returns
404, although every run reported success. The artifact deployment also names
the whole site as its target, which is the wrong mechanism for one
subdirectory.

**What replaces it.** `deploy-journal.yml` now publishes this app the way
`deploy-task-app.yml` publishes Trestle:

- on a push to `main` touching the Mosaic Journal, its runtime or its
  toolchain;
- the web host built with a **release** wasm (`JOURNAL_WASM_PROFILE=release`;
  the default stays debug for local development and tests);
- the bundle checked by `verify_relocatable_bundle.py` from a subdirectory;
- the wasm checked to be in the bundle and fetched by a **relative** URL;
- pushed with `peaceiris/actions-gh-pages` into `gh-pages` under `journal/`,
  with `keep_files: true` so sibling sites stay.

Two changes make the bundle relocatable, as #13832 did for Trestle: Vite's
`base` is `"./"`, and the page fetches `journal_mosaic_app.wasm` relative to the
document instead of from `/`.

**Data.** The old app kept its entries under its own storage keys, and this app
uses `journal-mosaic/state`, so neither can overwrite the other. Importing the
old app's entries stays deferred. The old app's code and its Electron release
(`release-journal.yml`) are unchanged; retiring them is a separate decision.

## Search (J4a)

A search field above the timeline turns it into search results. The engine
already does the searching (`journal_core::projections::search`: every term
must match, title and tag hits rank above body hits, then newest first, at most
16 terms from the first 1,024 characters, a snippet of at most 160 characters).
This adds the UI and nothing else.

**State.** `search_query: String`. It is **not** in the snapshot: search is a
way of looking at the journal, not part of it, so a restart opens the full
timeline. A query longer than the engine reads (1,024 characters) is refused
as it is typed, like an oversized draft, so it cannot grow the update echoed on
every keystroke.

**Slots** (added to the table above):

| slot | type | value |
| --- | --- | --- |
| `search-query` | `text` | the field's value |
| `searching` | `bool` | the query has a non-blank character |
| `no-matches` | `bool` | `searching` and no entry matches |

While `searching`, `timeline-rows` holds the hits in rank order instead of the
day groups:

- `heading` is `""`, because results are ranked, not grouped by day;
- `title` and `badge` are as for a timeline row;
- `subtitle` is the engine's snippet around the first body match;
- `meta` is the entry's day in short form (`24 Sep 2026`), so a hit still says
  when it was written. It is short because `meta` shares a line with the
  title button in the 300px pane: rendered on Compose, the long heading form
  ("Thursday, 24 September 2026") did not fit beside a title and was drawn
  over it. *Changed from the first draft, which reused the heading form.*

`timeline-empty` stays "the journal has no entries". A search that matches
nothing is `no-matches`, not `timeline-empty`, so the two empty states can say
different things.

**Events** (added to the table above):

| event | payload |
| --- | --- |
| `onSearchChange` | `{ value }` |
| `onClearSearch` | — |

`onSelectEntry`'s `index` is a row of whatever `timeline-rows` was last
rendered: a result while searching, a timeline row otherwise. Selecting,
saving and deleting do not clear the query. The results are recomputed, so an
entry edited so that it no longer matches drops out.

**Layout.** In the pane, under *New entry*:
- a `Row [ search-bar ]` holds a `HostInput [ search-input ]` (placeholder
  "Search", accessible name "Search entries") and, only while `searching`, a
  *Clear* button;
- then `EmptyState` "No entries yet" when `timeline-empty`;
- else `EmptyState` "No entries match" when `no-matches` (a second
  `EmptyState` mount, #15959);
- else `RecordList`.

## Stars (J4b)

The engine stores `Entry::starred` (`Command::SetStarred`) and filters on it
(`EntryFilter::starred_only`). Rows already show a starred entry's `★` badge.
This adds the two controls.

**State.** `starred_only: bool`. Like the search query, it is a way of looking
at the journal, so it is **not** in the snapshot.

**Slots:**

| slot | type | value |
| --- | --- | --- |
| `star-label` | `text` | `"Star"` or `"Unstar"` for an existing entry, `""` for `New` (so the button is hidden) |
| `starred-only` | `bool` | the filter is on (the filter button shows as selected) |
| `no-starred` | `bool` | `starred-only`, not searching, and no entry is starred |

`starred-only` applies to the timeline **and** to search results: both use
`EntryFilter { starred_only, .. }`. A search that finds nothing is still
`no-matches`, whether the filter is on or off. `no-starred` covers only the
plain timeline, so each empty state says the right thing.

**Events:**

| event | payload |
| --- | --- |
| `onToggleStar` | — stars or unstars the entry in the editor; an error for `New` |
| `onToggleStarredFilter` | — |

Toggling a star changes only `starred`. The draft is not saved with it, so an
unsaved title edit stays unsaved (Cancel still reverts it). With the filter on,
unstarring the open entry drops it from the list but keeps it in the editor.

**Layout.**
- **Pane:** under the search bar, a *Starred only* toggle button (`selected`
  bound to `starred-only`, UI86). Then the list's empty states, in order:
  - "No entries yet";
  - "No entries match";
  - a third `EmptyState`, "No starred entries", when `no-starred`.
- **Editor:** above `DraftEditor`, the *Star* / *Unstar* button when
  `star-label` is set.

## On this day (J4c)

Day One's best-loved screen: what you wrote on this date in earlier years. The
engine already answers it (`journal_core::projections::on_this_day`: entries
on today's month and day in earlier years, most recent year first, with a
29 February entry recalled on 28 February in a non-leap year). This shows the
answer above the timeline.

**Today** is the user's local day, as for filing entries (`today(clock, UTC
offset)`), so the recall follows the user's clock, not UTC's.

**Slots:**

| slot | type | value |
| --- | --- | --- |
| `on-this-day-rows` | `list<list<text>>` | `RecordList` rows for the recalled entries |
| `has-on-this-day` | `bool` | there are recalled entries and no search is running |

Rows use the timeline row shape. `heading` is set on the first row of each
year, as `1 year ago · 24 Sep 2025` (or `N years ago · …`). `meta` is `""`
because the heading already says when. The *Starred only* filter applies here
too, since every projection shares one `EntryFilter`. During a search the
section is hidden and the results take the pane.

**Event:**

| event | payload |
| --- | --- |
| `onSelectOnThisDay` | `{ index }`, a row of the `on-this-day-rows` last rendered |

It opens the entry in the editor exactly as `onSelectEntry` does. It has its
own event because each `RecordList` indexes its own rows. The open entry is
highlighted in both lists, since both bind `selected-key`.

**Layout.** In the pane, above the timeline's empty states and list: when
`has-on-this-day` is set, an "On this day" heading and a second `RecordList`
mount. The layout already mounts `RecordList` once; the resolver renames the
second mount's parts (#15959).

## Tags (J4d)

The engine stores an entry's tags (`Command::SetTags`, which tidies them and
drops case-insensitive duplicates), counts them (`tag_counts`) and filters on
one (`EntryFilter::tag`). J4d adds a way to write tags and a way to filter by
them.

**Writing.** The editor gains a *Tags* field. Tags are typed comma-separated
("travel, family") and belong to the **draft**, like the title and body:
- `draft_tags: String` is in the snapshot (`#[serde(default)]`, so older
  version-1 snapshots still load);
- selecting an entry loads its tags as `travel, family`, Cancel reverts them,
  and Save writes them;
- Save validates the tags with `normalize_tags` **before** writing anything, so
  a bad tag (too long, a control character, more than 64 tags) fails the whole
  Save and leaves the journal unchanged. Only then does it run
  `CreateEntry` / `EditEntry`, followed by `SetTags`.

The field is capped as it is typed at what 64 tags of 64 characters could
take, so it cannot grow the update without bound.

**Showing.** A timeline or On-this-day row's `meta` is its tags, as
`#travel #family`, at most 24 characters (cut with `…`). It is short for the
reason search `meta` is: `meta` shares a line with the title button in a 300px
pane. A search row's `meta` stays the day.

**Filtering.** When any entry has a tag, the pane shows the tags as a
`SegmentedControl` of options such as `#travel (3)`, most used first
(`tag_counts`). The options count every entry, not the filtered list, so they
never vanish under the filter they set. Selecting a tag filters the timeline,
search and On this day (`EntryFilter::tag`), and selecting it again clears the
filter. `tag_filter` is a view, like `starred_only`, so it is not persisted. A
selected tag that no longer exists (its last entry was deleted or retagged)
stops filtering.

**Slots:** `draft-tags` (`text`), `tag-options` (`list<text>`),
`selected-tag-index` (`number`, `-1` for none), `has-tags` (`bool`).

**Events:** `onTagsChange { value }`, `onSelectTag { index }`, where index is
an option of the `tag-options` last rendered.

## An entry's day, and draft errors (J4e)

An entry is filed under the day it was written. J4e lets the user move it
(a late entry about yesterday, or a back-dated diary). The engine already has
`Command::SetEntryDate` and `Date::parse_iso`, which reads years 0 to 9999
and real calendar days only.

**The draft gains a day.** An editor *Date* field (`YYYY-MM-DD`) belongs to
the draft, like the title, body and tags:
- `draft_date: String` is in the snapshot (`#[serde(default)]`);
- selecting an entry loads its day, New leaves it blank, and Cancel reverts it;
- a blank date means **today** (the user's local day, as before), so the
  common case needs no typing.

On Save the date is checked **before anything is written**, like the tags.
Then Save runs `CreateEntry` with that day, or `EditEntry` plus
`SetEntryDate` when the day changed, followed by `SetTags`. The field is
capped at 32 characters as it is typed.

**Draft errors are shown, not thrown.** Save now reports a bad date or a bad
tag the way Trestle reports a bad due date (#14013): the update succeeds, the
journal is unchanged, and a `draft-error` slot says what to fix:
- "Use a real date in YYYY-MM-DD format."
- "Tag 2 is too long (64 characters at most)."

J4d returned an error from `dispatch` instead. That was correct for the
journal, which stayed unchanged, but silent for the person typing. Any event
other than Save clears the message, and the next Save sets or clears it
again.

**Slots:** `draft-date` (`text`), `draft-error` (`text`, `""` when there is
nothing to fix).

**Event:** `onDateChange { value }`.

**Layout.** In the editor: a *Date* field beside the *Tags* field, and the
error text above `DraftEditor` when set.

## Journals (J4f)

Day One keeps several journals (Personal, Work, Travel). The engine already
does too: `CreateJournal` (names unique case-insensitively, at most
`MAX_JOURNALS`), and entries that each belong to one journal
(`EntryFilter::journal`). Until now Journal only ever used the built-in
"Personal".

**Switching.** The pane shows the journals as a `SegmentedControl` (a second
mount, after the tags): "All journals", then each journal by name in the
order it was created. Selecting a journal filters the timeline, search, On
this day and the tag counts (`EntryFilter::journal`). "All journals" clears
the filter. `journal_filter` is a view, like the other filters, so it is not
persisted. A selected journal that no longer exists stops filtering.

**Creating.** Under the switcher, a *New journal* field and an *Add journal*
button run `CreateJournal`:
- the id is minted as `journal-{n}`, skipping any already in use;
- the name is the field's text, trimmed;
- a blank name does nothing;
- a name the engine refuses (a duplicate, too long, or too many journals)
  is said in words in `journal-error`, like `draft-error`;
- on success the field clears and the new journal is selected.

**Filing.** A new entry is created in the selected journal, or in Personal
when "All journals" is selected. Moving an existing entry between journals
is later work.

**Slots:** `journal-options` (`list<text>`), `selected-journal-index`
(`number`, `0` for All journals), `new-journal-name` (`text`),
`journal-error` (`text`).

**Events:** `onSelectJournal { index }`, `onNewJournalNameChange { value }`,
`onAddJournal`.

## An entry's journal (J4g)

The editor says which journal the draft belongs to, and Save can move it.
The engine already has `MoveEntry`.

**The picker.** Above the Date field, when there is more than one journal,
the editor shows a *Journal* `SegmentedControl` (a third mount, after the
pane's two). It lists each journal by name, in the pane's order but without
"All journals", with the draft's journal selected. Choosing another option
changes only the draft; choosing the selected one again does nothing.

**The draft's journal.** `draft_journal` joins the draft (in the snapshot,
`#[serde(default)]`, so older snapshots still load):
- a new draft starts in the pane's selected journal, or Personal under "All
  journals" (this replaces J4f's rule, and says the same thing);
- opening an entry sets it to the entry's journal, and Cancel reloads it;
- Save files a new entry there, and runs `MoveEntry` for an existing entry
  whose journal changed. As with the date, the move is checked before
  anything is written, so a refused Save writes nothing;
- a draft whose journal no longer exists (a restored snapshot) falls back
  to the entry's own journal, or Personal for a new draft.

An entry moved out of the journal the pane is filtered to leaves the
timeline but stays open in the editor, like unstarring under *Starred only*.

**Slots:** `draft-journal-options` (`list<text>`), `draft-journal-index`
(`number`), `has-journals` (`bool`, more than one journal).

**Events:** `onDraftJournalChange { index }`.

## An empty journal (J4h)

A journal can be empty while others are not: a newly added one, or one
whose entries were moved away. Until now the pane then showed nothing below
the filters, as if the list had failed to load.

A fourth `EmptyState` mount says so instead: "No entries in this journal",
with "New entries you write while it is selected are filed here." It shows
when a journal is selected, that journal has no entries, and there is no
search (a search that finds nothing is `no-matches`, as before). The pane
picks one empty state, in this order:

1. `timeline-empty`: no entries in any journal ("No entries yet");
2. `no-matches`: searching, and nothing matches;
3. `journal-empty`: the selected journal has no entries;
4. `no-starred`: *Starred only*, and nothing starred;
5. otherwise the timeline.

`journal-empty` comes before `no-starred` because it is the more accurate
reason: with no entries there is nothing to star.

**Slots:** `journal-empty` (`bool`).

## Renaming and deleting a journal (J4i)

The engine has `RenameJournal` and `DeleteJournal { move_entries_to }`.
Under the *New journal* row, while a journal (not "All journals") is
selected, a row offers **Rename** and **Delete journal**.

**Rename** gives the selected journal the name in the *New journal* field,
trimmed, and clears the field. A blank name does nothing. A name the engine
refuses (a duplicate, too long, or several lines) is said in
`journal-error`, the same words as for Add. Personal can be renamed; its id
does not change.

**Delete journal** removes the selected journal and **moves its entries to
Personal**. It never deletes entries, so no confirmation is needed. The
pane then shows "All journals", and a draft filed in the deleted journal
falls back as J4g's `draft_journal()` already does. Personal itself cannot
be deleted: restoring a snapshot requires it, so the button is not offered
while Personal is selected.

**Slots:** `can-rename-journal` (`bool`: a journal is selected),
`can-delete-journal` (`bool`: a journal other than Personal is selected).

**Events:** `onRenameJournal`, `onDeleteJournal`.

## Export (J6a)

An **Export** button saves the whole journal to a file the person chooses,
through Mosaic's standard `files.save` effect (UI87 §7). The app writes no host
code: Compose answers it with its platform library, the browser with
`mosaic-file-effects.mjs`, and a backend without a platform library yet fails
the effect, which the app reports.

- **Event:** `onExportJournal`. The runtime emits one `Await` effect:

  ```text
  files.save {
    suggestedName: "journal-YYYY-MM-DD.json",    -- the user's today
    accept: ["application/json"],
    bytes: base64(<export file>)
  }
  ```

  While it is outstanding, a second Export does nothing, and the button is
  disabled (`exporting`).
- **The file** is Journal's own versioned JSON, so a later Import can load it
  with the same validating `restore` (UI47 §8.3):

  ```json
  { "format": "coding-adventures-journal",
    "schema": "journal-mosaic-app/state", "version": 1,
    "state": { ...the snapshot state... } }
  ```

  `state` is the whole snapshot state, so it includes the editor's unsaved
  draft (title, body, tags, date, journal) exactly as the browser already
  stores it; that is what lets Import restore the app as it was. The bytes
  are captured when Export is pressed (UI47 §8.3). Files larger
  than the 16 MiB `files.save` limit are refused before the effect, with a
  message.
- **Results.** `ok { name }` shows "Exported to NAME"; with `download: true`
  (the browser fallback) it shows "Downloaded NAME". `cancelled` shows
  nothing. `failed { message }` shows "Couldn't export: MESSAGE". The status
  line is `export-status`; it is not persisted.
- **Persistence while exporting.** A pending Await blocks snapshots (UI47
  §8.1), so hosts defer autosave until the export completes, including for
  edits made meanwhile; the answer's update saves everything at once. The
  journal is unchanged by an export either way. A host-supplied name or
  message loses control and bidirectional/zero-width format characters before
  it reaches the status line.

Markdown export (readable, one-way) and Import are later steps.

## Deferred

Launching on the remaining native lanes, packaging and release (J5); the markdown preview;
(search J4a, stars J4b, on-this-day J4c, tags J4d, an entry's day J4e and journals J4f an entry's journal J4g, an empty journal J4h and renaming and deleting a journal J4i are above; export J6a is above); import; Markdown export; importing the TypeScript app's entries; export on SwiftUI, Qt, XAML and Flutter, which arrives with their platform libraries (UI87 §7.3).
