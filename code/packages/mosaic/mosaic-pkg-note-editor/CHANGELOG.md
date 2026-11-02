# Changelog

## Unreleased

### Changed — 0.2.0: the focused field is a body-only DraftEditor (E4)

E4 of `mosaic-editor-convergence-audit-v1.md` (probe B; decisions in §8).

- **The focused field is `pkg::mosaic-pkg-draft-editor::DraftEditor`**,
  mounted with `title-label`, `save-label`, `delete-label` and
  `cancel-label` all `""`, so it draws no title and no buttons. `body-label`
  is `selected-field-label`, `body-value` is `selected-field-value`, and
  `body-placeholder` is `selected-field-placeholder`. NoteEditor's own
  pickers, tags field and Save note / Delete / Cancel row are unchanged.
- **Behaviour changes.** The field is multi-line: Enter is a line break
  (it did nothing before). Ctrl+Enter (Cmd+Enter on macOS) raises
  `onSaveNote`, through DraftEditor's `onSave`.
- **Accessible names.** The focused field is named by its visible label
  (DraftEditor binds the body's `a11y-label` to `body-label`). `tags-input`
  gains `a11y-label : slot: tags-label`. Neither field had a name before
  beyond its placeholder. A host must never send `selected-field-label: ""`.
- **No slot or event change.** All 25 slots and 8 events are as in 0.1.0, so
  a consumer's call site does not change. Parts removed:
  `selected-field-label` and `selected-field-input` (now DraftEditor's
  `draft-editor-body-label` and `draft-editor-body`), with their styles in
  both themes.
- **Declared dependency:** `mosaic-pkg-draft-editor = "0.2.0"`, the first
  with `body-placeholder`. The manifest and the smoke crate's `Cargo.toml`
  are both 0.2.0 (the crate said 0.1.0).

### Changed — the native gate pins by part, per theme, and counts

- Pins are `(backend, property, part)`, as in `mosaic-pkg-notes`' E2 gate.
  The inverse ratchet runs per theme, and a new test keeps the pin list free
  of duplicates, so its length is the exact per-theme drop count.
- **Measured 1 → 4 per theme**, identical in dark and light. Kept: Flutter
  `font-weight` on `editor-title`. Added by the nested mount (audit R2):
  SwiftUI `gap` on `draft-editor-title-group` (a reporter artifact: the
  part is folded away at compile time), and Qt and Flutter `min-height` on
  `draft-editor-body` (DraftEditor's own pinned gap). Flutter `width: 100%`
  on `draft-editor-body`, under this package's Row, also dropped until
  #18467 bounded such children with `Expanded`; it now lowers. Capability
  degradations stay at zero.
- Mutation-tested: moving the SwiftUI `gap` pin to another part fails the
  forward test (the real drop is unpinned) and the inverse test (the pin
  matches nothing), in both themes.

### Changed — tests and stories

- `package_compiles.rs` asserts the body-only mount and that the parts it
  replaced are gone, the declared dependency and version, the multi-line
  field with its Ctrl/Cmd+Enter save and its placeholder (on a React build,
  where the mount is inlined), both accessible names, and that the layout's
  only literals are the `""` that switch DraftEditor's parts off.
- Four populated stories replace the one empty `Default`: adding a note,
  editing a multi-line field, confirming a delete, and an empty collection
  with no field to select. A test checks that each story sets exactly the
  declared slots, that none leaves the field unnamed, and that the empty
  field's placeholder and a multi-line value are both shown.

- Tests read the backend list from `mosaic_package_artifact_builder::Backend` (`Backend::NATIVE` / `Backend::ALL`) instead of a hand-kept copy, so a backend added to Mosaic is gated here automatically.

### Fixed — 6 stale style-drop pins removed (X-7)

The emitters now lower these, so the gate's inverse ratchet
(`no_pinned_style_drop_has_silently_been_fixed`) failed on every main CI run.
Removed from `ALLOWED_STYLE_DROPS`: `(Qt, "padding-bottom")`, `(Qt, "padding-top")`, `(Flutter, "border-radius")`, `(Flutter, "color")`, `(Flutter, "padding-bottom")`, `(Flutter, "padding-top")`. Comments that only explained
removed pins went with them.

### Fixed — the Flutter `font-size` pin was licensing a drop that no longer happens

Flutter now projects numeric typography onto its text controls, so
`(Flutter, "font-size")` stopped occurring, and
`no_pinned_style_drop_has_silently_been_fixed` correctly failed on it. The
pin is removed. Found when a Qt emitter change (plain-text `Text`) brought
this package's gate back into CI.

### Added — a native-complete gate for this package

`package_compiles.rs` proves the sources round-trip through the three IR
compilers and deliberately says nothing about what a backend can express. So a
capability this package asked for and a backend could not provide was invisible
here — it showed up, if at all, only once Engram composed the package and its
app-level gate noticed.

This package now runs the real degradation analyzer itself, across all five
native backends. A failure names one package, one backend, one property,
instead of reporting one signal for eleven packages at once.

It remains **capability-clean** on all five backends. Style-drop reporting now
also covers all five: measured pre-existing Qt and Flutter losses are pinned by
backend/property, while every unlisted loss still fails and every stale pin is
rejected.

**Both themes are analysed, which the sibling gates do not do.** `theme: None`
selects the historical dark-wins default, so a property dropped only by the
light stylesheet is invisible to it. Measured rather than assumed: adding
`flex-wrap` to a light `.msl` left the analyzer reporting nothing, and the
identical line in the dark `.msl` produced two drops. `mosaic-pkg-toolkit`'s and
`engram-app`'s gates both pass `None` and so cover one theme each.

Mutation-tested, not assumed green: a `flex-wrap` added to a light stylesheet
now fails with `SwiftUI (light)` and `Xaml (light)` naming the property and the
reason.

The style half now covers all five native backends. Qt gained real lowering-read
recording in #15245 and Flutter in #12022; their measured pre-existing losses
are pinned by backend/property, and the inverse-ratchet test rejects stale pins
as those gaps close.


- Added a light-theme stylesheet (`NoteEditor.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Add `NoteEditor`, a reusable focused-field note editor component with field
  selection, selected field editing, tag editing, and save/delete/cancel emits.
- Add portable deck and note-type option lists so generated shells can create
  notes without a host-specific modal.
