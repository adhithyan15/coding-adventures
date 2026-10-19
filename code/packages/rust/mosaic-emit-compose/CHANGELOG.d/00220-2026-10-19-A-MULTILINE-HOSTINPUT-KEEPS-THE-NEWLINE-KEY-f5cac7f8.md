## 2026-10-19 (A multiline HostInput keeps the newline key)

- A multiline field with `onCommit` no longer sets
  `KeyboardOptions(imeAction = ImeAction.Done)` and `onDone`. Done replaced the
  soft keyboard's newline key, so a text area could not start a new line from
  the IME, and on no other backend does Enter commit a text area (#15930,
  UI29 §2.1). Single-line fields still commit from the IME as before.
