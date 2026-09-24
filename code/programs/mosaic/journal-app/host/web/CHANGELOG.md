# Changelog

## 0.1.0 — unreleased

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
