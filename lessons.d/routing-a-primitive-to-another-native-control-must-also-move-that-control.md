---
category: Mosaic compiler pipeline
---

# Routing a primitive to another native control must also move that control's import detection

**What went wrong.** #15930 taught Qt's `HostInput` to honour
`multiline: true` by passing the keyword to the shared
`emit_text_input_qml`, which then emits a `TextArea`. The emitter's unit test
passed: it checked for `TextArea {`, the binding, the name and the absence of
`onAccepted`. But whether the file imports `QtQuick.Controls` is decided
somewhere else, in `tree_needs_controls_import`, which asked about
`multiline` only for the legacy `Input` tag. A multiline `HostInput` without a
placeholder would have emitted a `TextArea` with no import. QML does not
degrade that; it refuses to load the file. It was caught before commit only
because a reread of the function turned up the second `multiline` check.

**Fix.** Ask the import question for both tags, and make the unit test assert
`import QtQuick.Controls` on a fixture with no placeholder, since only
`multiline` can pull the import in there. Falsified: the assertion fails with
the import fix reverted.

**Do differently.** When you route a node to a different native control, grep
the emitter for every other place that branches on the same tag *and* on the
same prop. Imports, `uses_*` feature flags, focus wrappers and capability
checks often live apart from the lowering itself. Then assert the dependency
(the import, the wrapper) in the test, not just the control's name. Better
still, run the platform's own checker on real output: `qmllint` for Qt and
`swiftc -typecheck` for SwiftUI both run locally on macOS.
