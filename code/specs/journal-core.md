# journal-core — the headless engine behind Journal (J2)

Issue: [#14416](https://github.com/adhithyan15/coding-adventures/issues/14416) (step J2) ·
Program: [#14415](https://github.com/adhithyan15/coding-adventures/issues/14415) ·
Prior step: [`journal-component-demand-audit-v1.md`](journal-component-demand-audit-v1.md) (J1)

## Why a Rust core

Journal today (`code/programs/typescript/journal-app`) keeps its whole model in a
React reducer. That was fine for one host. The Mosaic plan (#14415) ships every
product to nine backends, and a model that lives in one host's language has to be
rewritten for each of the other eight. Trestle solved this with `task-core`;
Engram with `engram-core`. Journal follows the same shape:

```
journal-core        ← this spec: PURE ENGINE — model, commands, projections
  └ journal-wasm    (J2b) JSON ABI for web / Electron / every Mosaic host
      …consumed by the Mosaic journal-app (J3)
```

J1 concluded that nothing it found touches the engine, so this spec is
independent of the presentation work that follows.

## House rules (identical to task-core)

- **Pure.** No I/O, no system clock, no randomness. The current time is passed in
  as `now_ms: u64`; ids are minted by the host (UUID v7) and passed in.
- **Zero dependencies by default.** `serde` is an opt-in feature for the host
  facade. The only path dependency is `datetime-core` for civil-date arithmetic.
- **Derived data is computed, never stored.** Timelines, on-this-day, search
  results and tag counts are projections over the stored state.
- `#![forbid(unsafe_code)]`.

## The model

### Journal

Day One's first structural feature that the TypeScript app lacks is **more than
one journal** ("Personal", "Work", "Travel").

| field | type | notes |
| --- | --- | --- |
| `id` | `JournalId` | string-backed |
| `name` | `String` | trimmed, 1–128 chars, unique case-insensitively |
| `created_at_ms` | `u64` | |

`JournalState` always holds **at least one** journal and at most 1,000.
`JournalState::new` checks its first journal exactly as `CreateJournal` would,
so it returns a `Result`. The last journal cannot be
deleted — a journal app with nowhere to write is a broken state, not an empty one.

### Entry

| field | type | notes |
| --- | --- | --- |
| `id` | `EntryId` | string-backed |
| `journal` | `JournalId` | must exist |
| `title` | `String` | may be empty; ≤ 512 chars |
| `body` | `String` | raw GFM markdown; ≤ 1 MiB |
| `date` | `Date` | the **calendar day** the entry belongs to |
| `created_at_ms` / `updated_at_ms` | `u64` | instants |
| `tags` | `Vec<Tag>` | ≤ 64, deduplicated case-insensitively, insertion order kept |
| `starred` | `bool` | Day One's "favourite" |

`date` is deliberately a civil date and not an instant — the same decision the
TypeScript app made with `createdAt: "YYYY-MM-DD"`. A late-night entry written at
00:30 about "today" belongs to the day the user says it does, and moving an entry
to another day (`SetEntryDate`) is a first-class edit, not a timestamp rewrite.

`Date` is `i32` days since 1970-01-01, exactly `datetime_core::Date` and
`task_core::Date`. Its wire form is an ISO `YYYY-MM-DD` string so the TypeScript
app's stored entries map across with no conversion.

### Ids

Ids are minted by the host, but the core still checks them: 1–64 bytes of
printable ASCII (a UUID v7 is 36). Ids are echoed into errors and host logs and
used as map keys, so an id carrying a newline or a terminal escape, or one
megabytes long, is refused (`InvalidId`, which deliberately does not echo it).

### Tag

A tag is a name, not an entity: Day One has no tag objects with their own ids.
A `Tag` keeps its **display form** (trimmed, inner whitespace collapsed to one
space) and compares by a **case-folded key**, so `Travel` and `travel` are the
same tag and the first spelling wins. 1–64 chars; no control characters.

"Control characters" here, and in journal names, also covers the invisible
formatting characters — zero-width spaces and joiners (U+200B–U+200F,
U+2060–U+2069), bidirectional overrides (U+202A–U+202E), and the byte-order
mark. They draw as nothing, so without this rule `Work` and `Wo​rk` would look
identical and still count as different names.

## Commands

Every mutation is a `Command` applied by `apply(&mut state, cmd, now_ms)`. A
rejected command leaves the state **unchanged** (commands validate before they
write) and returns a typed `OpError`.

| command | effect |
| --- | --- |
| `CreateJournal { id, name }` | new journal |
| `RenameJournal { id, name }` | |
| `DeleteJournal { id, move_entries_to }` | entries move to `move_entries_to`, or are deleted if `None`; refuses the last journal |
| `CreateEntry { id, journal, date, title, body }` | |
| `EditEntry { id, title, body }` | `None` leaves a field unchanged |
| `SetEntryDate { id, date }` | |
| `MoveEntry { id, journal }` | |
| `SetTags { id, tags }` | replaces the tag list (normalised, deduplicated) |
| `SetStarred { id, starred }` | |
| `DeleteEntry { id }` | |

Every successful entry command sets `updated_at_ms = now_ms`. Ids already in use
are rejected (`DuplicateId`) rather than overwritten — the host minted a
collision and should know.

## Projections

All projections take an `EntryFilter` (optional journal, optional tag,
starred-only) so "Work journal, tagged #travel" is one query everywhere.

- **`timeline(state, filter)`** → day groups, **newest day first**; within a day,
  newest `created_at_ms` first, ties broken by id so the order is total.
- **`on_this_day(state, today, filter)`** → entries from the same month and day
  in **earlier years**, grouped by year, most recent year first, each labelled
  with `years_ago`. An entry on 29 February is recalled on 28 February in
  non-leap years, so it is not silently invisible three years in four.
- **`search(state, query, filter)`** → case-insensitive, all-terms-must-match
  search over title, body and tags. Ranked by where terms matched (title and tag
  hits outrank body hits), then newest first. The query is read up to 1,024
  characters, repeated terms count once, and at most 16 distinct terms are
  used, since each term is a scan of every body. Each hit carries a snippet of at
  most 160 characters around the first body match, cut on character
  boundaries, never mid-codepoint.
- **`tag_counts(state, filter)`** → every tag with its entry count, most used
  first, then alphabetical.
- **`month_activity(state, year, month, filter)`** → `(day, count)` for each day
  of the month with entries — the data a calendar heat-map needs.

Dates are limited to the years 0–9999: exactly what the ISO wire form carries,
and far enough from `i32` limits that the civil-date arithmetic underneath
cannot overflow (`month_activity(i32::MAX, …)` returns an empty list).

Case folding uses Unicode `char::to_lowercase`, which can change a string's byte
length (`İ` folds to two chars). Search therefore keeps a folded-offset → original
-offset map rather than reusing folded offsets to slice the original text.

## Limits, and why they are enforced here

The core is fed by hosts that read JSON from disk and from import files. Every
length limit above is checked in the core so that no host can store an entry the
others refuse to load. Search snippets are bounded so a hostile or enormous body
cannot make a projection allocate proportionally to its size per hit.

## Out of scope for J2a

- **Persistence and import/export** — J2b's facade serialises `JournalState`
  and imports the TypeScript app's `Entry[]`. Deserialising does **not**
  validate by itself, so the facade must call `JournalState::validate` on
  everything it loads, and hosts must escape titles, snippets and tags when
  they render them (they are plain text, not markup).
- **Photos** — needs the `image` slot J1 found has no consumer; J4.
- **Encryption** — J4, at the storage layer, not in the model.
