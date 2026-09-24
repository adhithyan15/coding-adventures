# Changelog

## 0.1.0 — unreleased

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
