# Changelog

## Unreleased

- The native gate's comment now says Compose lowers the fields'
  `width: 100%` (since #15967, a text field outside a Row fills its parent).

### Changed — the fields fill the editor, and the body has a height

- The title and body fields take `width: 100%`, and the body takes
  `min-height: 160`. Rendering Journal on Compose (#14416) showed an empty
  entry as a 120px-wide box.
- The web, SwiftUI and XAML lower both. Compose lowers the height and, for
  now, silently ignores a percentage width; lowering it safely inside a Row
  is a separate emitter change.
- Qt and Flutter lower neither on a text field yet. Pinned in the native
  gate, both ways.

### Added — `DraftEditor` (0.1.0, J3b of #14416)

Journal's entry editor, Trestle's notes and Engram's focused field all need the
same surface: an optional title, a multi-line body, and labelled actions. None
could share one. `NoteEditor` is an Anki note form, and `Notes` has hard-coded
English and no accessible names.

- **Controlled:** every edit emits the whole new text, and Save commits it.
- **Labels are slots and double as accessible names.** An empty label removes
  its part.
- **The body is the legacy multi-line `Input`,** which J3b-pre (#15931) made a
  real, named text area on every backend. The package has no dependencies, so
  that `Input` can't be rewritten to a same-named component.
- **No body placeholder and no preview,** for the reasons in the spec.
- **Tests:**
  - The interface and layout are pinned, including the gating, the naming and
    childless buttons.
  - It builds on all eight backends.
  - The native gate runs on all five native backends and both themes, with
    zero capability degradations. Three pre-existing Flutter style gaps are
    pinned (#12022).
- **MosaicBook:** four stories.
