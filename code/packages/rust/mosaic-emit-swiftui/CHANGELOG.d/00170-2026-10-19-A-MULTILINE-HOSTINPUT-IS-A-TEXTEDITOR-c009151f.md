## 2026-10-19 (A multiline HostInput is a TextEditor)

`HostInput ( multiline : true )` now lowers to the same `TextEditor` as the
legacy multiline `Input` (#15930). Every HostInput used to go to the
`TextField` path, so the note-type editor's card templates were single-line
here. The shared lowering (`emit_text_editor`, renamed from
`emit_legacy_input`) gains `onCancel` as a macOS-guarded `.onExitCommand`,
as `TextField` has; Return is a line break in a `TextEditor`, so nothing is
wired to `onCommit`. `auto-focus` keeps working because its focus wrapper is
keyed on the HostInput tag.
