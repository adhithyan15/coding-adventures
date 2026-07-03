# mosaic-pkg-note-type-editor

Reusable Mosaic component for Engram-style note type editing.

`NoteTypeEditor` keeps model-editing UI out of the Engram app package. It
exposes a portable workflow for selecting note types, editing the model name and
stylesheet, inspecting field/template lists, and committing new/save/delete/
cancel actions. Generated HTML, Electron/React, SwiftUI, Qt, XAML, Flutter, and
Compose shells can use the same component while the shared Rust facade owns the
durable note-type reducer path. The package ships a target-neutral Mosaic
interface, layout, and Lattice style file rather than app-owned host UI.
