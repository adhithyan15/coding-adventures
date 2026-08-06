# task-app Notes — v1 scope (engine entity only)

Phase 8 of [task-app-super-app.md](task-app-super-app.md) calls for: "Notes
entity in the engine + `mosaic-pkg-notes` (adapted `note-editor`): standalone
notes and per-task/project notes." This spec splits that into two PRs — the
same "ship narrower, iterate" sequencing every other phase in this backlog
has used (Sheet shipped read-only before cell editing; Board shipped three
columns before richer treatment; Calendar shipped month-view-only before
week/day) — and covers **only the first half**: the engine entity, its ops,
and its WASM/JS surface. `mosaic-pkg-notes` (the UI) is a separate follow-up
PR, deliberately, so each PR stays reviewable and each layer gets verified on
its own before the next depends on it.

## Why split here specifically

Unlike Sheet/Board/Calendar (all pure-UI slices wired to an engine surface
that already existed), Notes needs **new engine model work first** — there
is no notes entity in `task-core` today (confirmed: only `Task.notes:
String`, a plain per-task field, not a first-class entity with its own id,
standalone existence, or attachment relationship). Landing the engine half
alone is a complete, independently-useful, independently-testable unit: it's
exercisable and verifiable via `cargo test` without any UI depending on it
yet, exactly like `calendar(range, view)` (#8726) shipped as an engine-only
PR before Calendar's UI (this PR) consumed it.

## What this PR ships

- **`Note` entity** in `task-core`: `id: NoteId`, `title: String`, `body:
  String`, `attached_task: Option<TaskId>`. `None` = standalone (belongs to
  the project itself); `Some(task_id)` = attached to that task. This covers
  both halves of the spec's "standalone notes and per-task ... notes"
  without inventing a separate attachment-kind enum — a project-level
  standalone note and a task-attached note are the same struct, differing
  only in whether `attached_task` is set.
- Stored per-project: `ProjectState.notes: BTreeMap<NoteId, Note>`,
  `#[serde(default)]` so every already-persisted IndexedDB workspace (which
  has no `notes` key at all) still deserializes — the same backward-
  compatibility discipline `ProjectState.labels` and `ProjectState.parent`
  already established when they were added.
- Ops: `upsert_note(&mut self, note: Note)` (create-or-replace, the same
  whole-entity-upsert shape `upsert_resource` already uses — no separate
  create/update split) and `delete_note(&mut self, id: &NoteId)`.
- `delete_task` **orphans** a task's attached notes rather than deleting
  them — sets `attached_task` back to `None` (making them standalone) for
  any note attached to the deleted task, mirroring how deleting a task
  already reparents its children rather than destroying them. A note is a
  first-class entity; deleting the task it happened to be attached to
  should not silently destroy content the user wrote.
- WASM export (`export_op!`) + **JS binding** for both ops. The research
  pass for this PR found a real, pre-existing gap: `set_notes` (the
  per-task `String` field setter) has a working WASM export but was never
  wired into `task-engine.mjs` — the Rust engine supports it end-to-end and
  the JS surface silently doesn't. This PR's own ops are checked against
  that exact mistake before merging (see the test plan).

## What's explicitly deferred to the follow-up PR

- **`mosaic-pkg-notes`** (the UI component), adapted from
  `mosaic-pkg-note-editor`. That package was built for Engram (an
  Anki-style flashcard app) — roughly a third of its 25 slots (note-type
  selector, deck selector, focused-field-list editing) are domain-specific
  to Anki's note-type/deck/multi-field model and don't apply to a plain
  title+body note; the reusable core is closer to "a new component
  inspired by NoteEditor's shell" than "NoteEditor minus two slots." That's
  real UI design/build work belonging in its own PR, not a mechanical port.
- **Rich text.** `task-app-super-app.md` §3 calls Notes "rich text," but
  today's `Task.notes` field and every other free-text field in this engine
  is plain `String`. `Note.body` ships as plain text in this PR (matching
  the existing convention); a rich-text storage format (Markdown string vs.
  a structured doc) is a real design decision for whoever builds the editor
  UI, not something to decide as a side effect of the engine-model PR.
- **A dedicated notes query/projection** (e.g. "all notes for task X," "all
  standalone notes in this project"). Every other whole-entity type in this
  engine (`resources`, `fields`, `labels`) has *no* dedicated query export
  either — the only way a host reads them back today is the catch-all
  `workspace()` query, which already includes `ProjectState.notes` for free
  once the field exists (no new query-export work needed for that). A
  bespoke `notes_for_task(id)`-style query is UI-pull work, added when the
  UI package actually needs it, not before.
- **Tags on notes.** `NoteEditor`'s `tags-value`/`onTagsChange` is generic
  and reusable, but task-app has no notes-tagging UI to drive it yet — a
  UI-layer decision, deferred with the rest of `mosaic-pkg-notes`.

## Backward compatibility

`notes: BTreeMap<NoteId, Note>` is additive with `#[serde(default)]`: an
already-persisted workspace with no `notes` key deserializes to an empty
map, and nothing about `task-wasm`'s `Workspace`-vs-`ProjectState`
shape-sniffing in `load()` changes (neither shape gains or loses the
`projects`/`tasks` keys that discriminator relies on). No new snapshot
version field or migration step is needed — this repo's engine has never
used one; per-field `serde(default)` additions are the established
mechanism (see `ProjectState.labels`, `ProjectState.parent`).
