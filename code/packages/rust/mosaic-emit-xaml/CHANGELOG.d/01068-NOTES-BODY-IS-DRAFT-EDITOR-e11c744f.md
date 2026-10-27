### Changed — TaskApp's Notes body is found as `draft-editor-body` (E3)

- `tests/task_app_focus_compiles_to_xaml.rs` found Trestle's multi-line
  Notes body by the part `notes-body-input`. Notes 0.3.0 mounts
  `DraftEditor` for its body, so the TextBox is now DraftEditor's
  `draft-editor-body`, matched by prefix (a later DraftEditor mount in
  TaskApp would rename it `draft-editor-body-m2`). It still asserts
  `AcceptsReturn="True"` and `TextWrapping="Wrap"`. Test-only; the emitter
  is unchanged.
