# Changelog

## 0.1.0 — unreleased

- **Export (J6a).** An `export-bar` row under the journal actions holds an
  *Export* button (`export-journal`, disabled while `exporting`), and an
  `export-status` line says what the last export did. Styled like *Rename*;
  the status is muted, not an error colour. Rendered on Compose, both themes.

- **Renaming and deleting a journal (J4i).** Under the *New journal* row, while
  a journal is selected, a `journal-actions` row offers *Rename* (Add's
  style; it uses the field's name) and *Delete journal* (in the draft
  error's red; it moves the journal's entries to Personal). Personal is
  never offered for deletion. New slots: `can-rename-journal`,
  `can-delete-journal`. New events: `onRenameJournal` and `onDeleteJournal`
  (the contract test counts 21).
- **An empty journal (J4h).** Before, a selected journal with no entries left
  the pane blank under the filters. Now a fourth `EmptyState` mount says "No
  entries in this journal" (between "No entries match" and "No starred
  entries" in the chain, so the starred state's parts are now `-m4`). New
  slot: `journal-empty`.
- **An entry's journal (J4g).** Above the Date field, when there is more than
  one journal, a labelled *Journal* `SegmentedControl` (the third mount, so
  the pane's options keep their part names) shows the draft's journal, and
  Save files or moves the entry there. New slots: `draft-journal-options`,
  `draft-journal-index`, `has-journals`. New event: `onDraftJournalChange`
  (the contract test counts 19).
- **Journals (J4f).** Under the tags, a second vertical `SegmentedControl`
  mount switches between "All journals" and each journal (after the tags, so
  the tag options keep their part names). Under it, a *New journal* field and
  an *Add* button, in the search bar's styles, and a red line when a name is
  refused. New entries go to the selected journal. New slots:
  `journal-options`, `selected-journal-index`, `new-journal-name`,
  `journal-error`. New events: `onSelectJournal`, `onNewJournalNameChange`,
  `onAddJournal`. The event contract test now counts 18.
- **An entry's day (J4e).** A *Date* field (`YYYY-MM-DD`, blank for today)
  above the Tags field moves an entry to another day, or files a new one on a
  past day. A red line above the editor says why Save refused a draft ("Use a
  real date in YYYY-MM-DD format.", "Tag 2 is too long …"). New slots:
  `draft-date`, `draft-error`. New event: `onDateChange`.
- **Tags (J4d).** A labelled *Tags* field above the editor (comma-separated,
  saved by Save), each row's tags as its `meta`, and, when any entry has a
  tag, a vertical `SegmentedControl` of options like "#travel (3)" (the first
  `SegmentedControl` in Journal; vertical because the pane is 300px). An
  option filters every list, and selecting it again clears the filter. New
  slots: `draft-tags`, `tag-options`, `selected-tag-index`, `has-tags`. New
  events: `onTagsChange`, `onSelectTag`.
- The native gate's comment no longer says Compose ignores the editor
  fields' `width: 100%`; it lowers it since #15967. The search field, a text
  field inside a Row, is the case Compose still leaves at intrinsic width.
- **On this day (J4c).** Above the timeline, when earlier years have entries on
  today's date: an "On this day" heading in the stars' amber, and a second
  `RecordList` mount grouped as "1 year ago · 24 Sep 2025". It follows the
  *Starred only* filter and hides during a search. New slots:
  `on-this-day-rows`, `has-on-this-day`. New event: `onSelectOnThisDay`.
  There is no `font-weight` on the heading, because Flutter cannot lower it
  on a `Text` and the colour already sets the heading apart.
- **Stars (J4b).** A *Star* / *Unstar* button above the editor (hidden for a
  new draft), and a *Starred only* toggle under the search field that narrows
  the timeline and search alike. With the filter on and nothing starred, a
  third `EmptyState` says "No starred entries". The toggle is two parts
  (`starred-filter`, `starred-filter-on`), as in Trestle's outline, so the "on"
  state has its own style on every backend. It uses the amber of the `★` badge.
  New slots: `star-label`, `starred-only`, `no-starred`. New events:
  `onToggleStar`, `onToggleStarredFilter`.
- The editor pane fills the window height like the timeline pane. In dark,
  it had stopped under its buttons and left white below.
- **Search (J4a).** A search field above the timeline turns it into ranked
  results from journal-core's `search`: every word must match, title and tag
  hits rank above body hits, and each hit shows a snippet and its day. A *Clear*
  button appears while searching. A search that matches nothing shows a second
  `EmptyState` ("No entries match"), distinct from "No entries yet". New
  slots: `search-query`, `searching`, `no-matches`. New events:
  `onSearchChange`, `onClearSearch`. The field is `width: 100%` in its row:
  Qt and Flutter drop that on a text field (already pinned), and Compose does
  not yet lower it for a leaf inside a Row (tracked).
- **Looked at on Compose.** `journal-mosaic-app/conformance/compose/
  JournalScreenshots.kt` renders Journal (empty, a draft, a saved entry, and
  a new entry beside the timeline), the same technique as Trestle's #14798
  harness. The timeline pane now fills the window height
  (`min-height: "100vh"`). DraftEditor's fields fill the editor where the
  backend lowers percentage widths.
- **Runs in the browser (J5c-2).** `host/web` is the web host: the generated
  `JournalApp` over the wasm runtime, persisted in `localStorage`.
  `scripts/build-web.sh` emits the React components and the wasm, and the new
  `journal-mosaic-web.yml` workflow tests it.
- **Builds on SwiftUI and Compose (J5b).** Their CI lanes emit this package
  with the strict standard binding, require zero degradations and no replaced
  files, then run `swift build` (macOS) and `gradle compileKotlin` (Linux).
- **Launches on Qt (J5a).** CI's Qt lane builds this package with the strict
  standard binding against `journal-mosaic-app`, installs it and launches it
  twice offscreen, restoring the second time. It also runs this package's
  tests, which nothing ran before, because the package opts out of the Rust
  workspace.
- `JournalApp` (J3c-2, #14416): a timeline (`RecordList` with an `EmptyState`)
  and a **New entry** button beside a `DraftEditor`, in a `HostNavigationSplit`.
- `JournalApp.mil` is exactly the slot/emit contract `journal-mosaic-app`
  implements. The tests check it both ways: props equal slots, and every emit
  is routed.
- Light and dark styles for the app's own parts, using only properties every
  native backend lowers. The shell's background was left out because Flutter
  does not lower it on the split view.
- A native-complete gate. It pins the Flutter `border-radius`, `color` and
  `font-size` drops inherited from DraftEditor, EmptyState and RecordList.
