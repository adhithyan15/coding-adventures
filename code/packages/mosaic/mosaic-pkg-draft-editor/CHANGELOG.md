# Changelog

## Unreleased

### Added — `body-placeholder` (0.2.0, E1 of the editor convergence audit)

- **`slot body-placeholder : text`**, bound to the body's `placeholder`: a
  hint shown only while the body is empty, and `""` shows none. Engram's
  focused field (its field name, or "Select a field") and Trestle's notes
  ("Write something…") both want one, and they are the next two consumers
  (`mosaic-editor-convergence-audit-v1.md` §4.1, probe A).
- **Breaking, hence 0.2.0.** The slot has no default, like every slot here,
  and a consumer that does not bind a package component's slot fails to
  build with `UnknownSlot` (audit R1). Journal, the one consumer, binds
  `body-placeholder : ""` in the same change and requires `0.2.0`.
- 0.1.0 left the placeholder out because a slot-bound one was literal-only
  on React, WebComponent and SwiftUI. EM-13 (2026-10-19) fixed that on every
  backend. The native gate is unchanged: no capability lost and no new style
  drop, in both themes.
- **MosaicBook:** every story sets `body-placeholder`, and a fifth story,
  "Empty note with a body placeholder", shows one on an empty body.
- **Tests:** the interface and layout pins include the slot and the
  binding, and a new test checks that each story sets exactly the declared
  slots and that one shows the placeholder. It reads the stories with the
  in-house `coding_adventures_bounded_json`, a new dev-dependency.

### Fixed — stale rationale in the package's own documents (audit R6)

- The spec's layout block named the body `Input`; it is `HostInput`.
- The spec, `.mil` and README gave a "no body placeholder" reason that EM-13
  made wrong.
- The native gate's header said the body is the legacy `Input`.
- The README said Trestle's notes share the package, and that Journal
  composes a markdown preview beside it. Journal is the only consumer, and
  it has no preview.
- The spec now says why the package stays outside the toolkit although its
  original reason is gone (audit R7).

- Tests read the backend list from `mosaic_package_artifact_builder::Backend` (`Backend::NATIVE` / `Backend::ALL`) instead of a hand-kept copy, so a backend added to Mosaic is gated here automatically.

### Changed — the body is `HostInput` and saves on Ctrl/Cmd+Enter (UI25 §4)

The body moved from the retiring legacy `Input` to the kernel text area,
`HostInput ( multiline : true )`, which lowers identically. Its `onCommit` is
`onSave`: Enter is a line break, and Ctrl+Enter (Cmd+Enter on macOS) saves, as
the Save button does. The chord raises `onSave` even when `save-label` is
empty; a host that offers no Save ignores it. Journal, the one consumer,
always offers Save. The package test pins `HostInput` and the commit. Verified
in Qt (`qmltestrunner`: bare Return no save, Ctrl+Return one save) and with
`swiftc -typecheck` for macOS 13.

### Changed — documentation only (#15930)

The layout comment notes that `HostInput ( multiline : true )` now lowers to
the same text area as the body's legacy `Input`, so the body can move to it
when `Input` retires. The generated output is unchanged.

### Fixed — 5 stale style-drop pins removed (X-7)

The emitters now lower these, so the gate's inverse ratchet
(`no_pinned_style_drop_has_silently_been_fixed`) failed on every main CI run.
Removed from `ALLOWED_STYLE_DROPS`: `(Qt, "width")`, `(Flutter, "width")`, `(Flutter, "border-radius")`, `(Flutter, "color")`, `(Flutter, "font-size")`. Comments that only explained
removed pins went with them.

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
