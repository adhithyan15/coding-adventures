# mosaic-pkg-collection-actions

A reusable Mosaic component package for collection-level study app actions.

The component is app-neutral: it receives formatted collection counts and action
labels, emits import/export and note/note-type workflow intents, and leaves file
picking, dialogs, persistence, and concrete core dispatch payloads to the host
app's shared business-logic layer.

Engram consumes this package from its app package so HTML, Electron, SwiftUI
macOS/iOS, XAML, Qt, and Flutter outputs expose the same collection workflow
contract instead of reimplementing the surface per target.
