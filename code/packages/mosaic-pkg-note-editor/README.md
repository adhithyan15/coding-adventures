# mosaic-pkg-note-editor

Reusable Mosaic component for Engram-style note editing.

`NoteEditor` keeps note-editor UI out of the Engram app package. It exposes a
portable focused-field workflow: the host supplies note/deck/model metadata, a
field label list, the selected field value, tags, and save/delete/cancel labels.
Generated HTML, Electron/React, SwiftUI, Qt, XAML, Flutter, and Compose shells
can use the same component while the host assembles the concrete `onSaveNote`
payload for `engram-core-wasm`.
