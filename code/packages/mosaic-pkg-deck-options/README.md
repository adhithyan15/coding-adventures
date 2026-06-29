# mosaic-pkg-deck-options

Reusable Mosaic controls for Anki-style deck scheduler options.

Engram owns the app and the Rust reducer. This package only owns the portable
UI contract for deck setting labels, learning/relearning step lists, numeric
values including Anki initial ease factor, Anki leech threshold/action controls,
sibling-bury checkboxes, and change events so HTML, React, Electron, SwiftUI,
Qt, XAML, and Flutter shells can share one settings surface.
Engram hosts can route those generated events through
`EngramSession::handle_engram_app_event`, which persists them with the shared
`setDeckOptions` reducer command.
