# mosaic-pkg-draft-editor — one plain editing surface (J3b)

Issue: [#14416](https://github.com/adhithyan15/coding-adventures/issues/14416) (J3b) ·
Program: [#14415](https://github.com/adhithyan15/coding-adventures/issues/14415) ·
Evidence: [`journal-component-demand-audit-v1.md`](journal-component-demand-audit-v1.md) finding 3

## Why

Three products edit "a short title and a long body, then save": Journal's
entry editor, Trestle's notes, and Engram's focused note field. Each built its
own, or could not:

- `NoteEditor` is an Anki note form (note types, decks, field lists) that
  contains a single-line field. J1 found almost nothing in it transfers.
- `Notes` (Trestle) has the right editing column, but it is welded to the note
  list and task attachment, hard-codes English labels, names no field for
  assistive technology, and has no native gate.
- Journal has none yet.

The program counts "Editor" at demand 3 (`mosaic-product-program-v1.md`). This is
the extraction J1 asked for: the plain surface underneath, not a generalised
note form.

## Why a package, not the toolkit

The only portable text area is the UI25 legacy `Input ( multiline : true )`.
Inside `mosaic-pkg-toolkit`, the artifact builder rewrites every bare tag that
matches a sibling export to that sibling (`qualify_local_component_references`),
so a bare `Input` in a toolkit component becomes the toolkit's *single-line*
`Input` component and `multiline` is silently lost. A package with no
dependencies reaches the primitive directly, as `Notes` and `SpiceWorkbench` do.

J3b-pre (#15931) made that primitive a real, named text area on every backend:
Flutter drew one line, Qt named it twice, React and SwiftUI dropped its name.

## Interface

```
component DraftEditor {
  slot title-label       : text ;  // "" = no title field; also its accessible name
  slot title-value       : text ;
  slot title-placeholder : text ;
  slot body-label        : text ;  // required; the body's visible label and accessible name
  slot body-value        : text ;
  slot save-label        : text ;  // each action is drawn only when labelled
  slot delete-label      : text ;
  slot cancel-label      : text ;
  emit onTitleChange ( value : text ) ;
  emit onBodyChange  ( value : text ) ;
  emit onSave ;
  emit onDelete ;
  emit onCancel ;
}
```

- **Controlled.** Every keystroke emits the whole new text; the host owns the
  draft and writes it back. Save commits. This is how all three hosts already
  work (Engram's draft buffer, TaskApp's `noteBodyDraft`).
- **Every label is a slot.** No English literals; the host localises.
- **Label-gated parts.** An empty `title-label` removes the title field (Engram's
  focused field has none); an empty action label removes that button, so the
  component can never draw an unnamed button.
- **No body placeholder.** The legacy primitive's `placeholder` is literal-only
  on React, WebComponent and SwiftUI (UI25), so a slot-bound one would vanish
  there. The visible label does the job.
- **No preview.** Rendered markdown is a `node` slot mounted with `HostSurface`,
  which is not styled by `.msl`, needs a surface per native host, and on the
  HTML backend is an unescaped triple-mustache. Keeping it out of this component
  keeps those costs with Journal, the one product that wants a preview.

## Layout

```
Column [ draft-editor ]
  If (title-label)  Column [ draft-editor-title-group ]
                      Text      [ draft-editor-title-label ]
                      HostInput [ draft-editor-title ]      (a11y-label: title-label)
  Text  [ draft-editor-body-label ]
  Input [ draft-editor-body ] (multiline: true, a11y-label: body-label)
  Row   [ draft-editor-actions ]
    If (save-label)   HostButton [ draft-editor-save ]
    If (delete-label) HostButton [ draft-editor-delete ]
    If (cancel-label) HostButton [ draft-editor-cancel ]
```

## Styles

Only properties every native backend lowers (the package's native gate allows
no capability degradation and pins every style drop). `Text` parts carry colour
and size only (#15276).

## Adoption

- **Journal (J3c)** uses it for new-entry and edit, with its own preview beside it.
- **Trestle `Notes`** swaps its editing column for it (follow-up PR).
- **Engram `NoteEditor`** could use it for the focused field (no title, no
  actions), but that changes Engram's field from single-line to multi-line and
  churns its pinned tests — a separate Engram PR.
