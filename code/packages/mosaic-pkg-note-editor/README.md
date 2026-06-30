# mosaic-pkg-note-editor

Reusable Mosaic component for Engram-style note editing.

`NoteEditor` keeps note-editor UI out of the Engram app package. It exposes a
portable deck/model selector, focused-field workflow, tags editor, and
save/delete/cancel controls. Generated HTML, Electron/React, SwiftUI, Qt, XAML,
Flutter, and Compose shells can use the same component while
`engram-core-wasm` owns draft note state and materializes cards through the
shared Rust core.
