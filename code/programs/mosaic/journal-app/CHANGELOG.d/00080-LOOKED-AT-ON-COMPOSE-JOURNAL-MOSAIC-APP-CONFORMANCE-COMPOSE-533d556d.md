- **Looked at on Compose.** `journal-mosaic-app/conformance/compose/
  JournalScreenshots.kt` renders Journal (empty, a draft, a saved entry, and
  a new entry beside the timeline), the same technique as Trestle's #14798
  harness. The timeline pane now fills the window height
  (`min-height: "100vh"`). DraftEditor's fields fill the editor where the
  backend lowers percentage widths.
