## 2026-10-19 (A TextEditor commits on Cmd/Ctrl+Return)

A multiline field's `onCommit` lowers to
`.modifier(_MosaicCommitChord { dispatch(...) })` (UI25 §4). The generated
helper uses `onKeyPress` for Return and keypad Enter, returns `.handled` only
with Command or Control held, and `.ignored` otherwise, so bare Return stays
the `TextEditor`'s line break and the chord inserts nothing. `onKeyPress` is
macOS 14 / iOS 17, above the generated package's floor (macOS 13 / iOS 16), so
the helper checks availability; on older systems the editor has no chord.

Chosen after probing a real AppKit-hosted `TextEditor`: `onKeyPress` fired for
Cmd+Return, Ctrl+Return and Cmd+keypad Enter and committed only the focused
editor of two; a hidden `.keyboardShortcut` button never fired while a
`TextEditor` was first responder. `.onSubmit` and the chord share
`text_commit_case_call`, so both carry the same payload.
