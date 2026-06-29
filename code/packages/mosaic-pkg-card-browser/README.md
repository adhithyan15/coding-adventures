# mosaic-pkg-card-browser

A reusable Mosaic component package for Anki-style card browser/search surfaces.

The component is app-neutral: it receives formatted search/query/result slots,
emits browser actions, and leaves persistence, search execution, tag mutation,
and card edits to the host app's shared business-logic core.
Result rows also carry target-neutral card IDs, note IDs, template IDs, state
labels, flag labels, and selected-row metadata so generated native/web hosts can route
browser actions back into the shared core without scraping display text.
The flag picker uses target-neutral option slots plus a typed selected-value
event, allowing hosts to map Anki card flags without backend-specific widgets.
The tag editor row emits draft text changes plus add/remove actions; hosts can
bind those to note-owned tag commands while still targeting selected cards.

Engram consumes this package from its app package so HTML, Electron, SwiftUI
macOS/iOS, XAML, Qt, and Flutter outputs share the same browser interface
contract instead of reimplementing it per target.
