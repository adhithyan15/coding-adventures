# task-app Notes — v1 UI scope

Second half of Phase 8, following on from the engine entity
([task-app-notes-entity-v1.md](task-app-notes-entity-v1.md)). Scopes
`mosaic-pkg-notes` — a list-plus-editor component adapted from
`mosaic-pkg-note-editor`, wired into `TaskApp` as a sixth view.

## What v1 ships

- **A list + editor shell**: a left column listing every note (standalone
  and task-attached, in one flat list — task-app has no separate "browse
  standalone vs. per-task" UI in v1; a note's attachment is just data, not
  a navigation split) with a "+ New note" button, and a right column
  editing whichever note is selected (title, body, Save/Delete/Cancel).
- **Create, edit, delete** — the full CRUD the engine already supports
  (`upsert_note`/`delete_note`, shipped in the entity PR). Save always
  calls `upsertNote` (create-or-replace by id — the same op whether it's a
  brand-new note or an edit, matching the engine's own whole-entity-upsert
  shape); a new note gets a freshly-minted id on first Save, the same
  pattern `addTask`/`addProject` already use for `main.tsx`-minted ids.
- **Both themes**, palette matching the rest of TaskApp's tokens (this
  component has no design-mock reference — `design/ui-prototype.html`
  predates the notes entity — so it's styled to match the existing
  Sheet/Board/Calendar treatments rather than a specific mock).

## What's explicitly deferred

- **Per-task attachment picker.** v1's editor has no UI to set/change
  `attached_task` — a note created from the Notes tab is always standalone.
  Attaching a note to a specific task (e.g. from the task detail panel) is
  real UI work belonging to the "richer task rows" design-fidelity item
  already tracked in `BACKLOG.md`, not this PR.
- **Tags.** `NoteEditor`'s `tags-value`/`onTagsChange` is generic and
  reusable, but nothing in task-app's data model or UI has a notion of
  note-tagging yet — deferred with no engine-side blocker, purely a
  "nobody asked for it yet" scope cut.
- **Rich text.** Per the entity spec, `Note.body` is plain text; v1's
  editor is a plain multi-line text field, not a rich-text editor.
- **Search/filter over notes.** The list renders every note; no search box
  in v1 (mirrors how Sheet shipped without a saved-search picker in v1).

## A real kernel gap found while designing this

UI29's kernel `HostInput` is **single-line only** — the legacy `Input`
(UI25) primitive is what supports `multiline: true` (lowering to
`<textarea>` in the React backend), and that support was **not** carried
forward into `HostInput`; per `mosaic-emit-react`'s own doc comment, a
`multiline` toggle is meant to live in "a userland `MultilineInput`
component," which doesn't exist yet. A note body genuinely needs multiple
lines — a single-line body field would be a real UX regression, not just
an aesthetic gap.

**Decision**: use the legacy `Input` primitive directly for the body
field, with `multiline: true`. This is a disclosed, deliberate use of an
existing, still-functioning, still-emitter-supported primitive for a real
need — not a new pattern to spread, and not blocking on building a new
`MultilineInput` kernel-adjacent component as a detour from shipping
Notes. If/when a userland `MultilineInput` lands, this is the one call
site in the whole task-app codebase that would need updating.

## Package structure

`code/packages/mosaic/mosaic-pkg-notes/`, mirroring `mosaic-pkg-sheet`'s
file trio: `Cargo.toml`, `mosaic-package.toml`, `src/lib.rs` (doc-only),
`src/Notes.mil`, `src/Notes.mll`, `src/Notes.{light,dark}.msl`,
`tests/package_compiles.rs`. No dependency on another package — built
from kernel primitives plus the one legacy `Input` use above.

## TaskApp wiring

Same shape as Sheet/Calendar's integration: `slot notes-mode : text ;`,
`emit onShowNotes ;` added to the "choose a view" group, a sixth branch in
the segmented switcher (six uniquely-named buttons per branch — the
pattern is now List/Board/Sheet/Calendar/Notes/Timeline, six families ×
six branches = 36 button declarations, following the exact precedent
Calendar's own addition already established), a sixth `If`/`Else` content
branch, and `pkg::mosaic-pkg-notes::Notes ( ... )` embedded with every
slot/emit forwarded explicitly. `main.tsx` gains `notes-mode`, a selected-
note-id + title/body draft state (mirroring the composer's `newName`/
`newDue` pattern), a `noteRows()` derivation from `engine.workspace()`'s
`projects[activeProject].notes` (no dedicated query needed — the entity
spec already noted `workspace()` includes it for free), and
`showNotes`/`selectNote`/`newNote`/`noteTitleChange`/`noteBodyChange`/
`saveNote`/`deleteNote`/`cancelNote` dispatch cases.
