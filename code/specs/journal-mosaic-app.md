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

## Deferred

The web host (J5c-2), launching on the remaining native lanes, packaging and release (J5); the markdown preview;
search, on-this-day, tags and stars in the UI; the journal switcher; moving an
entry to another day; importing the TypeScript app's entries.
