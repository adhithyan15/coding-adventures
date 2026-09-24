# Changelog

## 0.1.0 — unreleased

- The Journal web host (J5c-2, #14416): the generated `JournalApp` mounted
  over the `journal-mosaic-app` wasm runtime. Snapshots persist in
  `localStorage`, stored as text, and an unreadable one is kept aside rather
  than lost. The clock shim never throws. The runtime's announcements go to a
  polite live region.
- Vitest against the real wasm checks four things: the empty state renders; an
  entry can be written and saved; a reload restores it; an unreadable journal
  is kept aside.
