- **Search (J4a).** A search field above the timeline turns it into ranked
  results from journal-core's `search`: every word must match, title and tag
  hits rank above body hits, and each hit shows a snippet and its day. A *Clear*
  button appears while searching. A search that matches nothing shows a second
  `EmptyState` ("No entries match"), distinct from "No entries yet". New
  slots: `search-query`, `searching`, `no-matches`. New events:
  `onSearchChange`, `onClearSearch`. The field is `width: 100%` in its row:
  Qt and Flutter drop that on a text field (already pinned), and Compose does
  not yet lower it for a leaf inside a Row (tracked).
