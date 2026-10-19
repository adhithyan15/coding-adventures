### Fixed — a multiline HostInput does not commit on Enter (#15930)

- A `TextBox` with `AcceptsReturn="True"` spends Enter on a line break, so the
  merged `KeyDown` handler no longer carries an Enter-to-`onCommit` branch for a
  multiline field. Escape still raises `onCancel`. Every backend now treats
  Enter in a text area the same way (UI29 §2.1).
