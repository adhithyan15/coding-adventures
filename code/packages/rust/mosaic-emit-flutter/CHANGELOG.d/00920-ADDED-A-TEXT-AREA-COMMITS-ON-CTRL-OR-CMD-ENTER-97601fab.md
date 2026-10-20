### Added — a text area commits on Ctrl/Cmd+Enter (UI25 §4)

A multiline `HostInput` or `Input` with `onCommit` wraps its `TextField` in
`CallbackShortcuts` bound to the generated `_mosaicCommitChords` helper:
Enter and numpad Enter, each with `control: true` and with `meta: true`. A key
event the framework handles never reaches the platform text input, so the
chord adds no line break; bare Enter still inserts one. A text-payload commit
carries `mosaicController.text`, the field's current value, as `onSubmitted`
does for a single-line field. `package:flutter/services.dart` is imported
once for the helper or the drag runtime.
