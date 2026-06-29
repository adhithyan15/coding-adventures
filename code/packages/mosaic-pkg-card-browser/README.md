# mosaic-pkg-card-browser

A reusable Mosaic component package for Anki-style card browser/search surfaces.

The component is app-neutral: it receives formatted search/query/result slots,
emits browser actions, and leaves persistence, search execution, and card edits
to the host app's shared business-logic core.

Engram consumes this package from its app package so HTML, Electron, SwiftUI
macOS/iOS, XAML, Qt, and Flutter outputs share the same browser interface
contract instead of reimplementing it per target.
