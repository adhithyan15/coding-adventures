## 2026-10-19 (A text area commits on Ctrl/Cmd+Enter)

- A multiline `HostInput` or `Input` with `onCommit` now commits on a hardware
  Ctrl+Enter or Cmd+Enter (UI25 §4). The field's modifier gains
  `onPreviewKeyEvent`, which sees KeyDown `Enter`/`NumPadEnter` with
  `isCtrlPressed || isMetaPressed` before `BasicTextField` does and consumes
  it, so the chord adds no line break; bare Enter still inserts one, and the
  soft keyboard keeps its newline key. A text-payload commit carries the
  field's value, as the single-line `onDone` does.
- The key-event imports are gated on a drag runtime *or* a text-area commit,
  one gate per name, so a file using both imports each once.
