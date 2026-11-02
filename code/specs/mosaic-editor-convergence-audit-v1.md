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

   **Status (2026-10-27): blocked.** The package-level result matches probe
   C. With `notes-body` as the split (`pane-width : 220`,
   `collapse : never`), the old Row `gap` moved to a `padding-left` on a
   detail `Column`, and the three flex hints deleted, the gate measures **2
   drops per theme, down from 10**: only `draft-editor-body`'s `min-height`
   on Qt and Flutter remains. There are zero capability and zero behaviour
   degradations.

   The app breaks, though. Trestle mounts `Notes` inside its vertical
   `HostScroll [ content-scroll ]`, and two native lowerings of the split
   need a bounded height:

   - **Compose:** `NavigationSuiteScaffoldLayout` lays itself out at
     `constraints.maxHeight`. On the Notes tab, the emitted TaskApp's
     `TaskAppScreenshots` harness throws
     `Size(984 x 2147483647) is out of range`.
   - **SwiftUI:** a macOS probe measured `NavigationSplitView` at 0 points
     tall inside a vertical `ScrollView`, and at 776 points without one.

   No CI lane opens the Notes view natively, so CI would not catch either.
   PR-6 waits on one of two things. Either the split lowerings gain a
   bounded-height behaviour for a nested split, or Trestle gives the Notes
   view a bounded height outside `content-scroll`. Lesson:
   `a-hostnavigationsplit-nested-in-a-vertical-hostscroll-crashes-compose-and-collap`.
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
  package, and E4 must pin or fix both drops. (E4 pins the first; #18467
  fixed the second in the Flutter emitter, see D4.)
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

## 8. Addendum: E4 as built (`NoteEditor` 0.2.0)

**Status:** E4 is implemented on branch `feat/note-editor-on-draft-editor`.
It follows §4.2 and the §6 E4 row. Where those left a choice open, the
decision is recorded below with its reason. Measured on `origin/main` at
`4ff0435529e`.

### 8.1 What `NoteEditor` mounts

```
Column [ note-editor ]
  Text [ editor-title ]
  Row  [ note-metadata ]          note id, note type, deck (unchanged)
  Row  [ note-choice-row ]        note-type and deck option lists (unchanged)
  Row  [ editor-body ]
    Column [ field-list-column ]  one HostButton per field (unchanged)
    Box    [ focused-field-column ]
      pkg::mosaic-pkg-draft-editor::DraftEditor
        title-label : ""                      no title field
        body-label  : selected-field-label    visible label + accessible name
        body-value  : selected-field-value
        body-placeholder : selected-field-placeholder
        save-label, delete-label, cancel-label : ""   no actions
        onBodyChange → onFieldValueChange
        onSave       → onSaveNote             Ctrl/Cmd+Enter
      Text      [ tags-label ]
      HostInput [ tags-input ]    a11y-label : tags-label   (new)
  Row  [ editor-actions ]         Save note / Delete / Cancel (unchanged)
```

The parts `selected-field-label` and `selected-field-input` are gone. They
are now `draft-editor-body-label` and `draft-editor-body`. Every other
`NoteEditor` part keeps its name.

### 8.2 Decisions

- **D1. `NoteEditor`'s slot and event sets do not change.** DraftEditor's
  `body-label` and `body-placeholder` are fed from the existing
  `selected-field-label` and `selected-field-placeholder` slots, which already
  carry the field's name and hint. So the package gains no slot, and Engram's
  two layouts (`EngramApp.mll`, `EngramApp.touch.mll`) do not change. R1 (an
  added slot breaks every consumer that does not bind it) does not apply.
  `NoteEditor` already took every word as a slot, so no English moves out of
  the package. The `""` bindings for DraftEditor's title and action labels
  are not words: they are how a body-only mount switches those parts off.
  The version is still a minor bump (0.2.0), because the field's behaviour
  and its part names change, and it gains a package dependency
  (`mosaic-pkg-draft-editor = "0.2.0"`).
- **D2. The host sends "Field" when no field is selected.** §4.2 offered
  this or gating the editor on a selected field. This takes the first
  option: the field keeps a name in every state, and the layout does not
  switch at runtime. The placeholder in that state stays "Select a field",
  so the hint still says what to do. Gating would remove the field when a
  note type has no fields, and would also make the form's height jump on
  selection, which is a reachability risk in a native pane that does not
  scroll (lesson `a-component-swap-that-passes-every-compile-gate-...`).
  Every other state already sent the selected field's name.
- **D3. Ctrl/Cmd+Enter raises `onSaveNote`.** DraftEditor raises its `onSave`
  on the chord even when no Save button is drawn, so forwarding `onSave` to
  `onSaveNote` is all it takes. The host needs no change: the chord raises
  the same event as the Save note button, and the host handles it the same
  way.
- **D4. R2's nested drops: one pinned, one fixed in the emitter.** Both
  were measured, and neither loses anything 0.1.0 had:
  - *SwiftUI `gap` on `draft-editor-title-group`* is a reporting artifact.
    `title-label : ""` makes the title group's `If` a constant, so the
    package inliner folds it away. The part never appears in the emitted
    Swift, and the SwiftUI drop reporter reports the style of a part that has
    no occurrence as not lowered. Nothing on screen is missing. Fixing that
    is a SwiftUI reporter change that would also move other packages'
    gates, so it is filed as new work rather than done here.
  - *Flutter `width: 100%` on `draft-editor-body`* was a real constraint,
    now fixed. In `NoteEditor`, the field sits under `Row [ editor-body ]`,
    which gave its children unbounded width, and Flutter can fill 100% only
    of a bounded parent. #18467 (backlog 6d4, found while measuring E4) made
    the Flutter emitter wrap such a child in `Expanded`, which also stopped
    NoteEditor's 0.1.0 text fields throwing "InputDecorator cannot have an
    unbounded width". The width now lowers, and its pin is gone.
  - The body's `min-height` on Qt and Flutter is the drop DraftEditor's own
    gate already pins. Mounted here it is the same drop on the same part.
- **D5. `NoteEditor`'s gate pins by part, per theme, and counts.** It follows
  the E2 gate for `Notes`: each pin is `(backend, property, part)`, the
  inverse ratchet runs per theme, and the pin list's length is the exact
  per-theme drop count. A `(backend, property)` pin for `(SwiftUI, "gap")`
  would license any part of the package to drop `gap`.
- **D6. Engram's app gate gains part-scoped pins for the new kinds.** Its
  existing pins are by `(backend, property)` across the whole app. Two of
  the three nested drops are kinds that list does not pin: SwiftUI `gap` was
  removed from it on purpose, and Qt `min-height` was never there. Adding them as property pins would let any part of Engram drop them.
  So they go in a separate `(backend, property, part)` list, checked in both
  directions. Flutter `min-height` is already pinned by property.

### 8.3 Measured drops

Capability degradations stay at zero on all five native backends, in both
themes, for `NoteEditor` and for Engram.

`NoteEditor`'s style drops per theme (identical in dark and light):

| Backend | 0.1.0 | 0.2.0 | Added |
| --- | :-: | :-: | --- |
| SwiftUI | 0 | 1 | `gap` on `draft-editor-title-group` (D4) |
| Qt | 0 | 1 | `min-height` on `draft-editor-body` |
| XAML | 0 | 0 | |
| Flutter | 1 | 2 | `min-height` on `draft-editor-body` |
| Compose | 0 | 0 | |
| **Total** | **1** | **4** | |

The one 0.1.0 drop (Flutter `font-weight` on `editor-title`) is unchanged.
Probe B (§3.1) measured a fifth, Flutter `width` on `draft-editor-body`,
which #18467 fixed before E4 landed (D4).

Engram (`theme: None`, both layouts) reports six more style drops than
before: the same three, once for each of the two layouts that mount
`NoteEditor`.

### 8.4 R3: Engram's UI tests

A search of every conformance tree and native host for `selected-field-input`,
`selected-field-label`, `tags-input`, `note-field-option`, `focused-field` and
`save-button` finds no Engram test or host that types into the focused field
by its part tag. The one hit, `tags-input` in
`journal-mosaic-app/conformance/compose/JournalScreenshots.kt`, is Journal's
own part. Engram's hand-written XAML host applies props by name from the
core's JSON, so it needs no change.

### 8.5 What the user sees

- The focused field is multi-line, about 160 px tall on backends that lower
  `min-height`. Enter is a line break. It used to do nothing.
- Ctrl+Enter (Cmd+Enter on macOS) saves the note, as Anki's Add dialog does.
- The field draws inside DraftEditor's own frame (padding, border, and its
  label), so it looks like Journal's editor body, not like the tags field
  below it. DraftEditor's styles are its own, and a consumer cannot restyle
  them. This is the visual cost of one shared editor.
- A screen reader announces the field by its name, and announces the tags
  field as "Tags". Neither had an accessible name before.

### 8.6 Reachability in a default 800 x 600 window

The taller field makes the Add screen taller: measured on Compose desktop,
its content grows from about 565 to 733 px. Before Engram's screens could
scroll, that pushed the tags field and the Save note / Delete / Cancel row
out of a default 800 x 600 window with no way to reach them. That blocked
E4. The options were (a) give Engram's screens a scroll container, (b) shrink
DraftEditor's minimum height, which is a shared-package change, or
(c) accept the loss. The decision was (a), shipped as EN-6 (#18477): every
native backend but Qt scrolls the screen region, and #18481 (with its
content-sizing follow-up) brings Qt in.

Re-measured on this branch with Engram's Compose screenshot harness at
800 x 600, after scrolling the Add screen to its end: Tags at y 418 to 442,
and Save note, Delete and Cancel at y 512 to 548, all inside the window. The
header and screen switcher stay fixed above the scrolled region.
