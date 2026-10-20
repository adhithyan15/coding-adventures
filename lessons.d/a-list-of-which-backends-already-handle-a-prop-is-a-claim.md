---
category: Mosaic compiler pipeline
---

# A list of which backends already handle a prop is a claim; probe the emitted output before scoping the fix

**What went wrong.** EM-3's follow-up said a slot-bound `placeholder` was
literal-only on React, Web Components and SwiftUI, and the brief for the fix
listed HTML, Qt, XAML, Compose and Flutter as already binding it. Flutter did
not. Its `emit_host_input` read the hint with the same string-literal-only
`find_string_prop`, so `InputDecoration` had no `hintText` for every toolkit
`Input` and the note-type editor's fields. A fix scoped to the three named
backends would have closed the issue and left the fourth silent drop in
place.

**How it was caught.** Before editing anything I built four real consumer
packages (toolkit, note-type-editor, spice-workbench, card-browser) on all
eight backends with `mosaic-compile pkg … --backend <b>` and grepped each
output for the bound slot's camelCase name. Flutter declared
`frontTemplatePlaceholder` and never used it. The same probe also showed
`HostNumberInput`'s slot placeholder dropped on React, SwiftUI, Flutter and Qt
(filed as EM-14).

**Do differently.** When a task says "backend X already does this", treat it
as a hypothesis. Build a real consumer on every backend, save the output, and
check that the authored value appears where it should. A declared-but-unused
prop in the output is the signature of a silent drop. Keep the before/after
outputs: diffing them afterwards shows exactly which files the fix changed,
and that nothing else moved.
