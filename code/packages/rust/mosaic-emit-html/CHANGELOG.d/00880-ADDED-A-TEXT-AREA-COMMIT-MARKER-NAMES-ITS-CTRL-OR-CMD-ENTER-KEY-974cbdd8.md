- **Added: a text area's commit marker names its key (UI25 §4).** A multiline
  `HostInput` or `Input` with `onCommit` carries `data-on-commit` again, plus
  `data-commit-key="mod+Enter"`: it commits on Ctrl+Enter or Cmd+Enter, never
  on bare Enter, which is its line break. A field without the key marker
  commits on Enter, as before. The generated `main.js` binds it as
  `event.key === "Enter" && (event.ctrlKey || event.metaKey)` and prevents
  the chord's default. The README documents the marker for other hydrators.
