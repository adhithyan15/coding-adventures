## 2026-10-27 (TaskApp's Notes body is found as `draft-editor-body`)

`tests/task_app_focus_compiles_to_swiftui.rs` found Trestle's multi-line
Notes body by `.accessibilityIdentifier("notes-body-input")`. Notes 0.3.0
(E3 of the editor convergence audit) mounts `DraftEditor` for its body, so
the identifier is now DraftEditor's `draft-editor-body`, matched by prefix:
a later DraftEditor mount in TaskApp would rename it `draft-editor-body-m2`.
The `TextEditor` binding to `noteBodyValue` is still pinned, and the whole
emitted TaskApp still type-checks with `swiftc` on macOS. Test-only; the
emitter is unchanged.
