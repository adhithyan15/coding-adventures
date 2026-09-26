# Changelog

## 0.1.0 — unreleased

- **Export (J6a).** The host creates Mosaic's browser file executor
  (`createBrowserFileEffects`) with the runtime, runs `files.*` effects
  synchronously from the click that raised them (a picker needs the gesture),
  and saves the journal once no Await is outstanding (a pending effect blocks
  snapshots). A new test exports through a fake `showSaveFilePicker` and checks
  the file is Journal's versioned JSON (18 tests).

- **Published at /journal/ (J5d).** The owner decided this app replaces the
  old TypeScript Journal on the web. `deploy-journal.yml` now builds this host
  and pushes it into `gh-pages/journal/`, as Trestle is published. The old
  workflow published a Pages artifact that this site never serves, so
  `/journal/` had been a 404.
  - The bundle is relocatable: Vite `base: "./"`, and the page fetches
    `journal_mosaic_app.wasm` relative to the document instead of from `/`.
  - `JOURNAL_WASM_PROFILE=release` builds the runtime optimised for the site
    (670 KB); the default stays debug for tests and the dev server.
  - Checked by serving the release bundle from `/coding-adventures/journal/`:
    it boots with no console errors, and a saved entry survives a reload.
- **Renaming and deleting a journal (J4i)** needs no host code. A new test
  renames Work to Office, deletes it, finds its entry in Personal, and checks
  that Personal offers no *Delete journal*, through the real wasm.
- **An empty journal (J4h)** needs no host code. The journals test now
  checks that a newly added journal says "No entries in this journal"
  until an entry is written in it.
- **An entry's journal (J4g)** needs no host code. A new test checks that
  the picker is hidden while there is one journal. It then moves an entry
  from Personal to Work with the editor's picker, checks that Cancel after
  Save keeps the move, and finds the entry only under Work, through the
  real wasm.
- **Journals (J4f)** need no host code. A new test is refused the name
  "personal" (Personal already exists), adds *Work*, files an entry there,
  and switches between Personal, Work and All journals, through the real
  wasm.
- **An entry's day (J4e)** needs no host code. A new test is refused a
  2026-02-30 date with the message, then files the entry on 2026-09-01,
  through the real wasm.
- **Tags (J4d)** need no host code. A new test tags two entries, then
  filters by one tag and clears the filter, through the real wasm.
- **On this day (J4c)** needs no host code. A new test writes an entry under
  a 24 Sep 2025 clock, reloads under 24 Sep 2026, and finds it recalled and
  openable, through the real wasm.
- **Stars (J4b)** need no host code either. A new test stars an entry and
  toggles *Starred only* through the real wasm: first "No starred entries",
  then only the starred entry.
- **Search (J4a)** needs no host code: the generated component dispatches
  `searchChange` / `clearSearch` like every other event. A new test searches
  through the real wasm: two hits, then "No entries match", then *Clear*.
- **Entries are filed under the browser's local day.** The host passes the
  browser's UTC offset (`-new Date().getTimezoneOffset()`) as
  `StartContext.utcOffsetMinutes` (UI38 "Local time"). An offset outside
  −840..=840 is left out, so Journal falls back to UTC instead of failing to
  start. `loadRuntime` takes `{ now, utcOffsetMinutes }` for tests.
- The Journal web host (J5c-2, #14416): the generated `JournalApp` mounted
  over the `journal-mosaic-app` wasm runtime. Snapshots persist in
  `localStorage`, stored as text. An unreadable one is kept aside under its
  own key, never lost or overwritten. A tab stops saving once another tab
  changes the journal. The clock shim never throws. The runtime's announcements go to a
  polite live region.
- Vitest against the real wasm checks four things: the empty state renders; an
  entry can be written and saved; a reload restores it; an unreadable journal
  is kept aside.
