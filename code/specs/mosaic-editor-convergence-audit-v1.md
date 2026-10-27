# Mosaic editor convergence fit-audit (v1)

**Status:** PROPOSED. Nothing in this document has been built, and no code
changes with it.
**Audited:** 2026-10-26, against `origin/main` at `79322a77c56`.
**Program:** [#14415](https://github.com/adhithyan15/coding-adventures/issues/14415) ·
Journal: [#14416](https://github.com/adhithyan15/coding-adventures/issues/14416) ·
Backlog: [`MOSAIC-BACKLOG.md`](MOSAIC-BACKLOG.md) PR-8
**Method:** `mosaic-product-program-v1.md` §5. Adopt a package as it is, record
where it does not fit, and generalise only against that evidence.
**Follows:** [`journal-component-demand-audit-v1.md`](journal-component-demand-audit-v1.md)
(J1), whose finding 3 led to `DraftEditor`.

The product program counts "Editor" at demand 3: Trestle, Journal and Engram
all want one. Each product now has its own. This audit puts the three side by
side, says which one becomes the shared editor, and says what each consumer
has to change.

The lesson J1 recorded about its own first draft applies here too. Reading a
component's interface and stopping there is not an audit. So every
"it would fit" claim below was compiled as a probe against the real packages
(§3.1). Two of those probes found things that reading the interfaces did not
show (§5, R1 and R2).

## 1. Inventory

A search of `code/packages/mosaic` and `code/programs/mosaic` for `editor`,
`notes`, `draft` and `multiline` finds five components with a text-editing
surface. Three of them are general content editors and are the candidates.
The other two are recorded so that nobody counts them as editors later.

### 1.1 The three candidates

| | `Notes` (`mosaic-pkg-notes`) | `DraftEditor` (`mosaic-pkg-draft-editor`) | `NoteEditor` (`mosaic-pkg-note-editor`) |
| --- | --- | --- | --- |
| **Version** | 0.2.0. The smoke crate says 0.1.0. | 0.1.0 | 0.1.0 |
| **Built for** | Trestle's Notes view (`task-app-notes-ui-v1.md`) | J3b, as the shared surface ([spec](mosaic-pkg-draft-editor.md)) | Engram's Add/Edit screen |
| **Shape** | A note list and an editor, welded together | An optional title, a multi-line body, and actions | An Anki note form: note type, deck and field pickers, one focused field, and tags |
| **Slots** | 6: `notes-title`, `note-rows : list<list<text>>` (`[id, title]`), `selected-note-id`, `title-value`, `body-value`, `task-name-value` | 8: `title-label`, `title-value`, `title-placeholder`, `body-label`, `body-value`, `save-label`, `delete-label`, `cancel-label` | 25: note-id, note-type and deck labels, values and option lists; `field-labels`, `selected-field-index`, `selected-field-label`, `selected-field-value`, `selected-field-placeholder`; `tags-label`, `tags-value`, `tags-placeholder`; and the three action labels |
| **Events** | 8: `onSelectNote(index)`, `onNewNote`, `onTitleChange`, `onBodyChange`, `onTaskNameChange`, `onSave`, `onDelete`, `onCancel` | 5: `onTitleChange`, `onBodyChange`, `onSave`, `onDelete`, `onCancel` | 8: `onSelectNoteType`, `onSelectDeck`, `onSelectField(index)`, `onFieldValueChange`, `onTagsChange`, `onSaveNote`, `onDeleteNote`, `onCancel` |
| **Styled parts** | 17 | 10, all `draft-editor-*` | 35 |
| **Body field** | `HostInput (multiline: true)`. `onCommit` raises `onSave`, so Ctrl/Cmd+Enter saves. | The same | `HostInput`, **single-line**, with no commit binding |
| **Labels** | **Hard-coded English**: "+ New note", "Title", "Write something…", "Save", "Delete", "Cancel", and the empty-state hint | All slots | All slots |
| **Accessible names** | None. No input has an `a11y-label`; each relies on its placeholder. | Both fields use their label slot as `a11y-label` | None on `selected-field-input` or `tags-input` |
| **Optional parts** | Always draws Save, Delete and Cancel, including Delete on an unsaved new note | Each action is drawn only when it has a label; an empty `title-label` removes the title field | Always draws all three actions |
| **Consumers** | `task-app` only (`TaskApp.mll:537`), with every slot passed straight through | `journal-app` only (`JournalApp.mll:233`) | `engram-app` only (`EngramApp.mll:228` and `EngramApp.touch.mll:250`) |
| **Package native gate** | **None** | `tests/native_complete_gate.rs`: zero capability degradations on the 5 native backends in both themes, and 2 pinned style drops (Qt and Flutter `min-height`) | `tests/native_complete_gate.rs`: zero capability degradations, and 1 pinned drop (Flutter `font-weight`) |
| **Measured on native (§3.1)** | 0 capability degradations, but **12 unrecorded style drops per theme**: SwiftUI 3, Qt 6, XAML 1, Flutter 2, Compose 0 | Matches its gate | Matches its gate |
| **App-level gate** | TaskApp has none. `task-app/tests/package_compiles.rs` budgets only SwiftUI `min-width`. | `journal-app/tests/native_complete_gate.rs` pins the same two drops | `engram-app/tests/native_complete_gate.rs` |
| **MosaicBook** | No entries in `mosaicbook-degradations.json` | None | None |
| **Stories** | 1, `Default`, with empty fixtures | **4 populated**: Journal entry, editing a note, body only, empty new entry | 1, `Default`, with empty fixtures |
| **Tests** | 7 (`package_compiles.rs`) | 4 + 3 | 5 + 3 |
| **CI** | Yes, through the build tool (`BUILD`: `cargo test`) | Yes, and `journal-mosaic-web.yml` also triggers on it | Yes, through the build tool |

All three packages' tests pass on the audit base. This audit ran
`cargo test` in `mosaic-pkg-draft-editor`, `mosaic-pkg-note-editor` and
`mosaic-pkg-note-type-editor`, and the Notes probe in §3.1.

### 1.2 Not candidates

- **`NoteTypeEditor`** (`mosaic-pkg-note-type-editor` 0.1.0; 36 slots,
  14 events, 39 parts; Engram only; native gate with 4 Flutter pins; one
  empty story). It edits a note *model*: field names, card templates and a
  stylesheet. Its two `multiline` fields hold template source code. That is a
  code-editing need: monospace text, and probably no Ctrl+Enter save.
  Converging it with a prose editor would be the speculative generalisation
  §5 forbids. It is out of scope here.
- **`SpiceWorkbench`'s netlist field** (`mosaic-pkg-spice-workbench`). A
  `multiline` `HostInput` holding netlist source. No program hosts it (backlog
  SW). It is code, not prose, and it has no consumer.
- **VisiCalc** has no editor demand. Its formula bar and inline cell editors
  are single-line grid editors, a different component family (`mosaic-pkg-grid`).

## 2. Demand: what each consuming app needs

Taken from each app's layout and from the Rust adapter behind it
(`journal-mosaic-app`, `task-mosaic-app`, `engram-core-wasm`). Each need was
read from code, not inferred from the app's name.

| Need | Journal (`journal-app`) | Trestle (`task-app`) | Engram (`engram-app`) |
| --- | --- | --- | --- |
| **Title field** | Yes. Single-line, placeholder "Untitled", capped at 512 characters by the host. | Yes. Single-line. An empty title is shown as "Untitled" in the list. | **No.** One focused field at a time; the field's *name* is the label. |
| **Body** | Multi-line. Plain text the host treats as markdown. Capped at 1 MiB by the host. | Multi-line plain text. `Note.body` is plain by spec; not rich. | One Anki field value, **single-line today**. Anki fields are routinely multi-line. |
| **Rich text / preview** | Not in the Mosaic app. The old TypeScript app had a GFM preview. Rich text and photos are JO-3, blocked on UI88. | No (spec non-goal) | No. MathJax and image occlusion are EN-2 and EN-3, blocked on UI88. |
| **Controlled draft** | Yes. Host-owned; every keystroke emits the whole text. | Yes (`note_title`, `note_body`) | Yes (`NoteEditorSessionState`) |
| **Draft survives restart** | Yes (snapshot) | Yes (`TaskAppState` snapshot) | Yes (session snapshot) |
| **Autosave of the record** | No. Save is explicit. | No | No |
| **Word or character count** | No | No | No |
| **Body placeholder** | Not needed; the label suffices | "Write something…" (literal) | Yes, the field name, or "Select a field" (`selected-field-placeholder`) |
| **Save** | Yes, and Ctrl/Cmd+Enter | Yes, and Ctrl/Cmd+Enter | Yes ("Save note"). Anki's own Add dialog commits on Ctrl+Enter. |
| **Delete** | Only for an existing entry (`delete-label` is `""` for `New`) | Always drawn, even for an unsaved new note | A stateful label: "Discard draft", then "Delete note", then "Confirm delete" |
| **Cancel** | Yes | Yes | Yes |
| **Fields beside the editor** | Star, journal (`SegmentedControl`), date, tags, `draft-error`. All composed **outside** `DraftEditor`. | "Attach to task" (single-line, resolved by name on Save) | Note type, deck, field pickers, tags |
| **Error feedback** | `draft-error` slot, shown above the editor | **None.** An unknown task name makes Save silently do nothing (`task-mosaic-app/src/lib.rs`, `save_note`). | Events return errors |
| **Item list beside the editor** | Yes: `RecordList` in a `HostNavigationSplit` pane | Yes: Notes' own `For` of `HostButton`s | No (the browser is a separate screen) |
| **Localisable labels** | Labels are literals in the app layout, and the component allows that | Labels are literals *inside the package* | All slots |

Three things follow.

1. **The need everyone shares is small**: an optional single-line title, a
   multi-line body, host-controlled drafts, a commit chord, and actions that
   can each be switched off. That is exactly `DraftEditor`'s interface.
2. **Everything else is per-product, and composes beside the editor.**
   Journal already proves this pattern: star, journal, date, tags and the
   error message all sit outside `DraftEditor`.
3. **Nobody asks for word count, autosave or rich text today.** Autosave
   needs only `onBodyChange` and the host snapshot, which all three already
   have. Rich text is blocked on UI88 for both products that want it, so it
   is not an editor-component decision yet.

## 3. Fit matrix

● fits as is · ◐ fits after a small additive change · ○ does not fit

| Demand | `DraftEditor` | `Notes` | `NoteEditor` |
| --- | :-: | :-: | :-: |
| Optional title field | ● (`title-label: ""`) | ◐ always drawn | ○ none |
| Multi-line body | ● | ● | ○ single-line |
| Ctrl/Cmd+Enter commits | ● | ● | ○ |
| Controlled, whole-text change events | ● | ● | ● |
| Each action can be switched off | ● | ○ | ○ |
| Stateful action labels (Engram's confirm-delete) | ● (labels are slots) | ○ literals | ● |
| Body placeholder | ◐ needs `body-placeholder` (probe A) | ● literal only | ● |
| Accessible names | ● | ○ | ○ |
| No English inside the package | ● | ○ | ● |
| Extra fields composed outside | ● (Journal does it) | ○ task field built in | ○ tags built in |
| Item list | composed (`RecordList`) | built in | n/a |
| Note type, deck and field pickers | composed | n/a | built in (Engram only) |
| Package native gate | ● | ○ none | ● |
| Populated stories | ● 4 | ○ | ○ |
| Fewest unrecorded native drops | ● 0 | ○ 12 per theme | ● 0 |
| Word count, autosave, rich text | not demanded | not demanded | not demanded |

`DraftEditor` fits every shared demand except a body placeholder. It was
designed as the extraction J1 asked for, and its spec already says Trestle and
Engram should adopt it. On the audit base, neither has.

`Notes` and `NoteEditor` are not editors. Each is a **product form that
contains an editor**: Trestle's note browser, and Engram's Anki note form.
Their non-editor parts are real, product-specific demand: the list, the
attach-to-task field, and the type, deck and field pickers.

### 3.1 The probes

The probes were compiled in a scratch directory with copies of the real
packages. Each was built on all 9 backends with `build_package`, then analysed
with `analyze_package_degradations(…, NativeComplete)` on the 5 native
backends in both themes. Nothing was committed.

| Probe | What it changes | Builds | Capability degradations | Style drops per theme |
| --- | --- | :-: | :-: | --- |
| **A** | `DraftEditor` with `slot body-placeholder` bound to the body's `placeholder` | 9 / 9 | 0 | Qt `min-height` and Flutter `min-height`: the existing pins. **No new drop.** EM-13 (2026-10-19) made a slot-bound placeholder lower on every backend, so the spec's reason for having no body placeholder is gone. |
| **B** | `NoteEditor` with its focused `Text` and `HostInput` replaced by `pkg::mosaic-pkg-draft-editor::DraftEditor` (`title-label: ""`, no actions, `onBodyChange → onFieldValueChange`, `onSave → onSaveNote`) | 9 / 9 | 0 | The existing Flutter `font-weight` pin, the 2 `min-height` drops, and **2 new ones**: SwiftUI `gap` on `draft-editor-title-group`, and Flutter `width` on `draft-editor-body` (see R2) |
| **C** | `Notes` with its row loop replaced by `pkg::mosaic-pkg-toolkit::RecordList`, and its title, body and actions by `DraftEditor` (the task field stays) | 9 / 9 | 0 | 10, down from 12. The Qt `text-align` drops on the row buttons are gone. What remains is Notes' own `flex-grow`/`flex-shrink` split (SwiftUI 3, Qt 3, XAML 1, Flutter 1), which PR-6 (`HostNavigationSplit`) removes, plus DraftEditor's 2 pinned `min-height` drops. |
| **D** | Journal compiled against probe A's `DraftEditor`, **without** binding the new slot | **0 / 9** | n/a | `UnknownSlot: Unknown slot 'body-placeholder' referenced in layout — not declared in .mil` on every backend (R1) |

## 4. Recommendation (PROPOSED)

**`DraftEditor` becomes the one shared editor.** `Notes` and `NoteEditor` are
not retired. They are re-founded as compositions that mount `DraftEditor` for
their text surface and keep only the product-specific parts. This follows
`mosaic-component-program-v1.md` §9 (existing L3 packages are re-verified,
not rewritten) and product-program §5 (generalise against evidence).

### 4.1 What `DraftEditor` must gain

Only one thing, backed by Engram's demand and probe A:

- **`slot body-placeholder : text`**, bound to the body's `placeholder`.
  This is version 0.2.0.

Probe D shows that adding a slot breaks every consumer that does not bind it.
So this PR must also bind `body-placeholder` in Journal, its only consumer,
in the same change. That is the main reason to do this first, while there is
one consumer rather than three.

`DraftEditor` deliberately does **not** gain these:

- **Word count, autosave, preview or rich text.** Nobody demands them (§2).
- **An error slot.** Journal and Trestle both need error feedback, but Journal
  shows it above the editor, and composing it outside costs nothing. Promote
  it only if a consumer needs it *inside* the editor.
- **A single-line body mode.** A layout cannot switch `multiline` without
  re-creating the field, and Engram is better served multi-line (§2).
- **Extra-field regions.** Tags, task, date and journal belong outside, as
  Journal does today. The `$mosaic-child-slot` route is not lowered on the
  web backends anyway (`mosaicbook-degradations.json`, #14685).

### 4.2 What each consumer must change

- **Journal.** Bind `body-placeholder` (to `""`) when DraftEditor 0.2.0
  lands. Nothing else changes.
- **Trestle (`Notes` 0.3.0, `task-mosaic-app`).**
  - `Notes` mounts `RecordList` for its list and `DraftEditor` for title, body
    and actions. The attach-to-task field stays a `HostInput` beside it, and
    gains an `a11y-label`. The package's English literals move to its call
    site in `TaskApp.mll`, as Journal's are, or into slots.
  - The host widens `note-rows` to RecordList's
    `[key, heading, title, subtitle, meta, badge]`.
  - The host passes `delete-label: ""` for an unsaved new note.
  - `mosaic-emit-swiftui/tests/task_app_focus_compiles_to_swiftui.rs:154` and
    `mosaic-emit-xaml/tests/task_app_focus_compiles_to_xaml.rs:149` pin the
    part name `notes-body-input`. They move to `draft-editor-body`, matched by
    prefix (lesson
    `adding-an-earlier-mount-of-a-component-renames-the-later-mount-s`).
- **Engram (`NoteEditor` 0.2.0, `engram-core-wasm`).**
  - The focused field becomes a `DraftEditor` with no title and no actions;
    `NoteEditor` keeps its own pickers, tags and action row.
  - The field becomes **multi-line**, and Ctrl/Cmd+Enter raises
    `onSaveNote`, which is Anki's Add shortcut.
  - The host must never pass an empty `body-label`. Today an unselected field
    yields `""` (`unwrap_or_default`), which would leave the field without an
    accessible name. Use "Field", or gate the editor on a selected field.
  - `tags-input` gains an `a11y-label`.
  - Both Engram layouts (`EngramApp.mll` and `EngramApp.touch.mll`) are
    covered by the package change, because they mount `NoteEditor`, not
    `DraftEditor`.

### 4.3 Migration order

1. **E1, DraftEditor 0.2.0.** One consumer, so the breaking slot addition
   is cheapest now (R1).
2. **E2, a native gate for `Notes` as it is.** It pins today's 12 drops, so
   E3's improvement is measured rather than claimed.
3. **E3, `Notes` on `DraftEditor` and `RecordList`.** Trestle's fit is the
   closest: title, body and actions map one-to-one, and the host logic does
   not change.
4. **PR-6, `Notes` on `HostNavigationSplit`.** This removes the remaining
   flex drops. It is already in the backlog.
5. **E4, `NoteEditor` on `DraftEditor`.** Last, because it is the only step
   that changes behaviour: single-line becomes multi-line, and a new commit
   chord appears. It also brings the new nested drops that R2 describes.

E5 to E8 (§6) are independent of this order.

## 5. Misfits and risks

- **R1. Adding a slot to a shared package breaks every consumer that does
  not bind it** (probe D). An unbound package slot surfaces in the consumer
  as `UnknownSlot` on every backend. UI14 §2 says "unbound slots use zero
  values" for its `Grid` primitive, and nothing found in UI14, UI29 or
  `lessons.d/` says this differs for package components. The effect on
  shared components is severe: every additive change is a breaking change
  across all consumers in one PR, and once packages publish (PR-4) an
  additive minor version breaks consumers. This needs a decision (E6), and
  it bears on the publishing spec's version semantics.
- **R2. A package's own native gate does not certify it when nested.**
  `DraftEditor`'s gate is clean at its own root. Mounted inside
  `NoteEditor`'s `Box`/`Row` context, it drops SwiftUI `gap` (on a title
  group that `title-label: ""` removes at runtime) and Flutter `width: 100%`
  on the body. Each consumer's gate must still be run on the composed
  package, and E4 must pin or fix both drops.
- **R3. Engram changes behaviour.** A multi-line field changes Enter from
  "nothing" to "line break". Ctrl/Cmd+Enter is a new save path. Any Engram
  native UI test that types into the field by its part tag
  (`selected-field-input`) loses it. A search of the current conformance
  trees found none, but E4 must check again.
- **R4. Trestle's silent Save failure.** An unmatched task name makes Save a
  silent no-op. This is not a convergence problem, but E3 touches the same
  code and should not hide it. It is filed as E5.
- **R5. Trestle has no draft caps.** Journal refuses a title over 512
  characters or a body over 1 MiB as it is typed. Trestle accepts any size
  into the snapshot. That is a state-file growth risk shared with the
  release-state gate work. It is folded into E5.
- **R6. Stale rationale in `DraftEditor`'s own documents.** These claims are
  now wrong:
  - the spec's layout block still says `Input [ draft-editor-body ]`;
  - "No body placeholder … literal-only on React, WebComponent and SwiftUI" is
    stale since EM-13;
  - the gate's header says the body "is the legacy `Input`";
  - the README says the package "is shared by … Trestle's notes".

  `Notes`' manifest and `lib.rs` also still say "UI29's kernel HostInput is
  single-line only". All of these are corrected in E1 and E3.
- **R7. Placement.** `DraftEditor` lives in its own package only because a
  bare `Input` inside the toolkit was rewritten to the toolkit's single-line
  `Input`. The body is now `HostInput`, a kernel primitive, so that reason no
  longer holds. Moving it into the toolkit is possible, but it is churn with
  no consumer benefit while packages are unpublished. It is not recommended.

## 6. Follow-up work items (one PR each)

| ID | Work | Size | Depends on |
| --- | --- | :-: | --- |
| **E1** | `DraftEditor` 0.2.0: add `body-placeholder` (probe A), bind it in Journal in the same PR (R1), add a story for it, and correct the stale rationale in the spec, `.mil`, gate header and README (R6) | S | — |
| **E2** | Give `mosaic-pkg-notes` a `native_complete_gate.rs`, modelled on `DraftEditor`'s: both themes, zero capability degradations, and today's 12 drops pinned in both directions. Add populated stories. | S | — |
| **E3** | `Notes` 0.3.0 on `RecordList` and `DraftEditor` (probe C). Host: widen `note-rows`, `delete-label: ""` for an unsaved note. Literals move to the call site, the task field gets an `a11y-label`, the SwiftUI and XAML emitter tests match `draft-editor-body` by prefix, the gate's pins shrink, and the manifest and `lib.rs` comments are fixed. | M | E1, E2 |
| **PR-6** | (existing) `Notes` on `HostNavigationSplit`, removing the remaining flex drops | S | E3 |
| **E4** | `NoteEditor` 0.2.0: the focused field becomes a body-only `DraftEditor` (probe B), multi-line, and saves on Ctrl/Cmd+Enter. The host never sends an empty `body-label` and passes the placeholder through `body-placeholder`. `tags-input` gets an `a11y-label`. Pin or fix R2's two drops, add populated stories, and check Engram's UI tests (R3). | M | E1 |
| **E5** | Trestle note drafts: show an error when Save is refused (an unknown task), as Journal's `draft-error` does, and cap title and body as typed, as Journal does (R4, R5) | S | — |
| **E6** | Decide what an unbound package-component slot means (R1). Either it takes its zero value, as UI14 says for primitives, or every slot addition is a breaking version. Then make the compiler and the publishing spec agree. | S decision, M build | — |
| **E7** | Populated stories and `a11y-label`s for `NoteTypeEditor`'s inputs. The same isolation gap, outside this convergence. | S | — |

## 7. What this does not conclude

- It does not design rich text, preview or a markdown surface. Those are
  UI88's, and they are blocked for Journal and Engram alike.
- It does not converge `NoteTypeEditor` or SpiceWorkbench's netlist field.
  Both are code editors. A code-editor component (monospace text, perhaps
  line numbers) would be a separate, evidence-led item once a second code
  consumer exists.
- It does not retire `Notes` or `NoteEditor`. Their product-specific parts
  remain real demand from one product each.
- It does not claim the probes prove the migrations. They show the
  compositions build on all nine backends and measure their native drops.
  Behaviour (focus, commit chords, screen readers) is each follow-up's to
  test on real hosts.
