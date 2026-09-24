# Changelog

## Unreleased

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
