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

> **Update (#15930):** the gap below is closed. `HostInput` now takes
> `multiline: true` on every backend and lowers exactly as the legacy
> `Input ( multiline : true )` does (UI29 §2.1), so this call site can move
> to `HostInput` as part of retiring `Input` (U29-X1) — done: the body is now
> `HostInput ( multiline : true )` and saves on Ctrl/Cmd+Enter (UI25 §4). No userland
> `MultilineInput` is needed: a text area cannot be composed from
> single-line fields, so it belongs in the kernel.

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

## Addendum (v1.1): minimal attach-to-task control

Discovered while shipping the task-detail dependency list (a small,
spec-free PR — see `task-app`'s `CHANGELOG.md`): a task-detail "notes
paragraph" cell was drafted, reading
`Note.attached_task`, then pulled back out — `attached_task` exists on
every `Note` since the entity PR, but v1's editor (this doc, above)
never gave the user a way to *set* it. A read-only cell for a field
nothing can write is dead plumbing, not a shippable slice. This
addendum closes that gap with the smallest control that does, ship
together with the detail-panel cell it unblocks in the same PR.

**Scope decision**: a single-line "Attach to task" text field in the
Notes editor, next to title/body, resolving a typed *task name* to a
task id on Save — the exact same discipline the Sheet Labels column
(`code/programs/mosaic/task-app/CHANGELOG.md`'s label-management entry)
already established: case-insensitive exact-name match, and an
**unrecognised name rejects the whole Save** rather than silently
dropping the attachment or (worse) fuzzy-matching to the wrong task.
An empty field means "no attachment" — clearing the field and saving
detaches the note (`attachedTask: null`).

**What this deliberately is NOT**: not a dropdown/autocomplete picker,
not a multi-select, not editable from the task-detail side (attaching
happens only from the Notes tab — the detail panel is read-only,
matching every other detail-panel cell in `TaskApp.mll`). A real
picker UI (search-as-you-type, browse-by-project) is still the
"Notes: attachment picker" backlog item's fuller scope — this addendum
ships the minimal write path that makes the entity field usable at
all, not the final UX. Two task names colliding case-insensitively
across different projects is a pre-existing, undeduped condition (task
names, like project and label names, aren't unique workspace-wide) —
matching resolves within the *active* project's task list only, same
scope boundary `labelsByName()` already uses for labels.

**Wiring**: `Notes.mil` gains `slot task-name-value : text ;` and
`emit onTaskNameChange ( value : text ) ;`, rendered as one more
`HostInput` in the editor column. `TaskApp` passes both through
unchanged (same pattern as `title-value`/`body-value`) and owns a
`noteTaskName` draft string alongside the existing title/body draft.
On Save, the host resolves the typed name against the active
project's tasks (case-insensitive exact match); a miss aborts the
whole save with a console error, mirroring the Sheet Labels column's
`write()` contract. The task-detail panel's `detail-notes` cell
(`row[13]`, appended after the dependency list's `row[12]`) then reads
`Note.attached_task` for the open task — the read side this addendum
was blocked on.

## Addendum (v1.2): a refused Save says why, and drafts are bounded

Mosaic backlog item E5, from the editor convergence audit
(`mosaic-editor-convergence-audit-v1.md`, risks R4 and R5). Both hosts
had two gaps, and both are closed the way Journal already closes them.

**A refused Save is visible.** v1.1's "a miss aborts the whole save with a
console error" left the person typing with nothing to see: Save did
nothing, and only a developer console knew why (the native adapter,
`task-mosaic-app`, did not even log). Now a Save that names no task in the
active project:

- writes nothing, and keeps the whole draft — title, body and the typed
  task name — exactly as it was, so the person can correct the name;
- sets the new `note-error` slot to a one-line reason. The native adapter
  also announces it politely, as it announces "Note saved"; the web host
  has no announcement channel for either yet, so there the visible text is
  the whole signal;
- is cleared by **any other event** (typing, switching notes, changing
  view), and by the next Save that succeeds. A new Save sets or clears it
  again. Like Journal's `draft-error`, it describes the last attempt, not
  the data, so it is not part of the snapshot.

`note-error` is a TaskApp slot, not a `Notes` one. TaskApp renders it as
a `Text [ note-error ]` directly above the `Notes` mount, exactly as
Journal composes `draft-error` above `DraftEditor`. That keeps the change
additive: `Notes` gains no slot, so no other consumer of it has to bind
one (the unbound-package-slot question is the audit's E6), and E3's move
of `Notes` onto `DraftEditor` is unaffected. The audit records that
composing the error outside the editor "costs nothing".

**Drafts are capped as typed**, with Journal's limits and Journal's
semantics:

| Field | Cap | Measured in |
| --- | --- | --- |
| title | 512 | characters (Unicode scalar values) |
| body | 1 MiB (1,048,576) | UTF-8 bytes |

An edit past the cap is **refused, not truncated**: the event fails, the
draft keeps its previous value, and the field shows that value again.
Truncating would silently drop the end of a paste; refusing it shows the
person that the paste did not land. Bytes for the body, because the cap
exists to bound the state file, not to count prose.

**Stored notes are never cut down.** A note saved before this change may
be larger than the cap. It still restores, still opens in the editor
whole, and still saves. Journal can refuse such a snapshot outright,
because its caps have always existed; Trestle's have not. The one
difference from Journal follows from that: an edit to an over-cap draft is
refused only if it would make the draft *longer*. A legacy note can be
edited down towards the cap, but never grown past it.

The task-name field is not capped by this addendum. Task names themselves
have no limit anywhere in TaskApp yet, so a cap on the name typed to find
one would be arbitrary.

## Addendum (v1.3): `Notes` 0.3.0 is a composition of `RecordList` and `DraftEditor`

Mosaic backlog item E3, from the editor convergence audit
(`mosaic-editor-convergence-audit-v1.md` §4.2, probe C). `Notes` stops
drawing its own list rows, fields and buttons. It keeps only what is
Trestle's: the "+ New note" button, the attach-to-task field and the
empty-state hint.

**What `Notes` mounts.**

```
Column [ notes-root ]
  Text [ notes-heading ]
  Row [ notes-body ]
    Column [ notes-list ]
      HostButton [ notes-new ]                       new-note-label
      pkg::mosaic-pkg-toolkit::RecordList            rows: note-rows,
                                                     selected-key: selected-note-id
    If ( selected-note-id )
      Column [ notes-editor ]
        Text      [ notes-task-label ]               task-label
        HostInput [ notes-task-input ]               a11y-label: task-label
        pkg::mosaic-pkg-draft-editor::DraftEditor    title, body, actions
    Else
      Column [ notes-empty ]
        Text [ notes-empty-hint ]                    empty-hint
```

- **The list is `RecordList`.** `note-rows` widens from `[id, title]` to
  RecordList's row, `[key, heading, title, subtitle, meta, badge]`, which
  `Notes` passes through unchanged. Both hosts fill `key` and `title` as
  before (the id, and the title or "Untitled") and leave the other four
  empty, so a row is still one line. Selection stays by key
  (`selected-key: selected-note-id`); a click still reports the row's index,
  which the host resolves through the list it built, exactly as before.
- **Title, body and actions are `DraftEditor`** (0.2.0). Its fields gain
  visible labels, and each label is also the field's accessible name. The
  body keeps Ctrl/Cmd+Enter to save. Every `DraftEditor` slot is bound at
  `Notes`' call site, because an unbound slot of a mounted package component
  is a compile error (E6 F1).
- **The attach-to-task field stays `Notes`' own** `HostInput`, composed
  above the editor as Journal composes its tags and journal fields above
  its `DraftEditor`. It gains a visible label, `task-label`, which is also
  its `a11y-label`, so it no longer relies on its placeholder for a name.
  Above rather than below, because `DraftEditor` ends with its actions: a
  field after Save would be filled in after the button that commits it.
- **`Notes` draws no English.** Every literal becomes a slot of `Notes`,
  bound at TaskApp's call site, as Journal binds `DraftEditor`'s:
  `new-note-label`, `empty-hint`, `title-label`, `title-placeholder`,
  `body-label`, `body-placeholder`, `task-label`, `task-placeholder`,
  `save-label`, `delete-label` and `cancel-label`. None has a default:
  a default would put the English back in the package.
- **Delete is offered only for a saved note.** `delete-label` is the one
  label TaskApp does not write as a literal. It binds a new TaskApp slot,
  `note-delete-label`, which both hosts fill with "Delete" when the open
  note exists in the project and with `""` for a new note that has never
  been saved. `DraftEditor` draws no Delete button for `""`. Before this,
  Delete on an unsaved note only discarded the draft, as Cancel does,
  while the native host announced "Note deleted" for a note that never
  existed. The `onDelete` handlers stay as they are: a Delete that
  arrives anyway is still harmless.

**What does not change.** The eight events and their payloads; the host
logic for New, Select, Save, Delete and Cancel; the caps and the refused-Save
message (v1.2); `note-error` above the mount. `Notes` is 0.3.0 because its
slot set changes, and a consumer that does not bind the new slots no longer
compiles.

**Native drops.** The package gate (E2) pinned 12 style drops per theme,
named by backend, property and part. The row buttons' two Qt `text-align`
drops go with the rows. The body's `min-height` drops move from
`notes-body-input` to `DraftEditor`'s own `draft-editor-body`, which its own
gate already pins. `Notes`' list/editor flex split (`flex-grow` on
`notes-editor` and `notes-empty`, `flex-shrink` on `notes-list`) is
untouched; PR-6 (`HostNavigationSplit`) removes it.
