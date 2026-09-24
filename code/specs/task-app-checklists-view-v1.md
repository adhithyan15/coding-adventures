# Trestle Checklists view v1 — the surface (C3)

Issue: [#14018](https://github.com/adhithyan15/coding-adventures/issues/14018) (C3) ·
Engine: [`task-app-checklists-v1.md`](task-app-checklists-v1.md) (C1) ·
Component: [`mosaic-pkg-checklist.md`](mosaic-pkg-checklist.md) (C2) ·
Program: [#14415](https://github.com/adhithyan15/coding-adventures/issues/14415) §2

## What it is

A sixth view in Trestle's switcher, **Checklists**, beside List, Board, Sheet,
Calendar and Notes. It is where the standalone Checklist app's work moves to.
C1 put templates and runs in task-core; C2 built the run component. C3 wires
them into the Rust app (`task-mosaic-app`) and the Mosaic package
(`programs/mosaic/task-app`).

Checklists belong to a project (`ProjectState.checklists`), so the view shows
the **active project's** templates and runs, as every other view shows only its
own project.

## Scope, and the split

C3 is three PRs, so each can be reviewed alone:

| item | what | depends on |
| --- | --- | --- |
| **C3a** | the Rust app: view, library, running a run, and authoring flat templates. Also the `.mil`/`.mll` surface and the tests | C1, C2 (#15947), the Qt signal fix (#15948) |
| C3b | the web host (TypeScript over task-wasm), to the same contract | C3a |
| C3c | authoring decisions in a template (both branches, visible and labelled) | C3a |

This document specifies C3a in full and fixes the contract C3b must match.

**C3b (the web host).** `programs/mosaic/task-app/host/web` is its own
TypeScript controller over `task-wasm`, not the Rust app. It implements this
view to the same contract:

- the same slots, events, ordering, bounds, id scheme and progress wording;
- the same switcher position;
- the same guards: a calendar drop refuses a key the views do not show.

task-wasm gains the one op the port needed, `set_order`. The web host keeps
no checklist state of its own beyond the selection and the composers, which
are UI state and are not persisted, like its Notes drafts. Its Vitest suite,
`__tests__/checklists.test.ts`, walks the same scenarios as the Rust tests.

## The screen

```
┌ library ─────────────────┐ ┌ detail ───────────────────────────────┐
│ [ New checklist… ] [Add] │ │ template: outline, [ Add item… ][Add] │
│ Templates                │ │           [Start run] [Delete]        │
│   Release                │ │ run:      ChecklistRun (C2)           │
│   Pre-flight             │ │           [Delete]                    │
│ Runs                     │ │ nothing:  "Pick a checklist, or       │
│   Release · 3 of 5 done  │ │            make one."                 │
└──────────────────────────┘ └───────────────────────────────────────┘
```

- **Library**: a `RecordList` over `checklists()`. Templates come first, then
  runs. Each group is headed with RecordList's flattened grouping (the heading
  appears only on the group's first row). Runs are newest first.
- **Detail** shows the selection:
  - For a **template**, its outline: `checklist_outline`, flat in C3a, so
    each row shows its indent and name. Below the outline are an *Add item*
    composer, **Start run** and **Delete**.
  - For a **run**, `ChecklistRun` (C2), then **Delete run**.
  - With **nothing** selected, a short prompt.

  Both empty states (an empty library, and nothing selected) are plain text,
  not the toolkit `EmptyState`. The list view already mounts one, and an
  inlined component keeps its part names, so a second instance fails the
  package build (`DuplicatePart 'empty-state'`). *Changed from the first
  draft, which used `EmptyState` for both.*

## State (in `TaskAppState`)

| field | serialized | meaning |
| --- | --- | --- |
| `selected_checklist: Option<ChecklistId>` | yes, `#[serde(default)]` | the library selection |
| `new_checklist_name: String` | yes, `#[serde(default)]` | the *New checklist* composer |
| `new_checklist_item: String` | yes, `#[serde(default)]` | the *Add item* composer |

`#[serde(default)]` keeps every earlier snapshot, including the v0.1.0 upgrade
fixture, restoring. The snapshot schema stays `task-mosaic-app/state` v1.
`ViewMode::Checklists` is appended to the enum, because declaration order is
snapshot history. `repair()` clears a selection that names no checklist in the
active project. Switching project clears the selection.

**The clock.** Every checklist op takes `now`. `TaskMosaicApp` gains
`clock: fn() -> u64` (milliseconds since the epoch). It defaults to the system
clock and is replaced in tests, as in `journal-mosaic-app`. The clock is not
serialized.

## The switcher

`SWITCHER_ORDER` becomes List, Board, Sheet, Calendar, Notes, **Checklists**,
Timeline. Timeline stays last, so a Board-tier project, which gets every view
but Timeline, never has another view's index shifted. Checklists is offered at
both tiers: a checklist is as useful in a Board-tier project as in a Full one.
`switcher_views(full)` returns all seven for Full, and the first six otherwise.

## Props

| slot | type | value |
| --- | --- | --- |
| `checklists-mode` | text | `"checklists"` in this view, else `""` |
| `checklists-title` | text | `"Checklists"` |
| `checklist-library-rows` | list<list<text>> | RecordList rows `[key, heading, title, subtitle, meta, badge]`; see below |
| `checklist-library-empty` | bool | the project has no checklists |
| `selected-checklist-key` | text | the selection's id, `""` for none |
| `new-checklist-name` | text | composer value |
| `checklist-template-mode` | bool | a template is selected |
| `checklist-run-mode` | bool | a run is selected |
| `checklist-outline-rows` | list<list<text>> | `[key, indent, name]` for the selected template |
| `new-checklist-item` | text | composer value |
| `checklist-run-title` | text | the run's name |
| `checklist-run-progress` | text | e.g. `"3 of 5 done · 1 of 2 answered"`, or `"Completed"` / `"Abandoned"` once finished |
| `checklist-run-rows` | list<list<text>> | C2's rows, from `checklist_run(..).rows` |
| `checklist-complete-label` | text | `"Complete"` only when the run is in progress **and** `progress.complete`; else `""` |
| `checklist-abandon-label` | text | `"Abandon"` while in progress; else `""` |

All the run and template slots are empty when not relevant. The library, outline
and run rows are computed only in this view, as Notes' rows are.

Library rows:

- `title` is the name.
- A template's `subtitle` is its item count, e.g. `"4 items"`.
- A run's `subtitle` is its progress, as `checklist-run-progress`.
- `badge` is `✓` for a completed run and `✗` for an abandoned one.
- `meta` is `""`. Dates wait for the time-zone work.

Indent is two spaces per depth. The rows never carry the markers C2 rejects.

## Events

The wire names are the raw emit names. Every error leaves the app unchanged:
`dispatch` already clones and restores.

| event | payload | effect |
| --- | --- | --- |
| `onShowChecklists` | — | view → Checklists |
| `onSelectChecklist` | `{index}` | a library row just rendered |
| `onNewChecklistNameChange` | `{value}` | composer |
| `onCreateChecklist` | — | `create_checklist_template` with the trimmed name. An empty name is a no-op. Selects the new template and clears the composer |
| `onNewChecklistItemChange` | `{value}` | composer |
| `onAddChecklistItem` | — | `create_task` under the selected template's root, then clears the composer. An empty name is a no-op; with no template selected, the event is an error |
| `onStartChecklistRun` | — | `instantiate_checklist(selected template, new run id)`, then selects the run |
| `onChecklistToggle` | `{index}` | `set_completed(!completed)` on that visible row. A decision row is refused |
| `onChecklistAnswerYes` / `onChecklistAnswerNo` | `{index}` | `answer_decision` on that row. A check row is refused |
| `onCompleteChecklistRun` | — | `complete_checklist_run` (the engine refuses an incomplete run) |
| `onAbandonChecklistRun` | — | `abandon_checklist_run` |
| `onDeleteChecklist` | — | `delete_checklist` on the selection, then clears the selection |

`onShowView` also covers the new index.

**Bounds.**

- Both composers refuse input past 512 characters as it is typed, so no draft
  can grow the state file without bound.
- An added item takes the next `order` under the root. The engine sorts
  siblings by `(order, id)`, and minted ids do not sort numerically
  (`task-10` sorts before `task-9`).
- *Add item* is refused once the template holds `MAX_CHECKLIST_ITEMS`. A
  template the engine would refuse to instantiate can therefore never be built.
- A row's indent is capped at 16 levels. A restored snapshot can nest items
  arbitrarily deep, and N chained items would otherwise cost N² bytes of indent
  on every render.
- The library's item counts come from task-core's one-pass `checklists()`
  (`ChecklistSummary.items`), not from an outline per template. The library is
  rebuilt on every event in the view, keystrokes included.
- Every minted id (task, project, label, note, checklist, run) uses the
  checked counter. A counter at `u64::MAX` fails the event rather than wrapping.
- Board and Calendar drops refuse a checklist item's key. Those views never
  show one, and `set_constraint` has no checklist guard.

Answering the same answer again clears it (`clear_decision_answer`), so a
mis-tap can be undone. The TS app did this by tapping the chosen answer. In a
finished run, the engine refuses every tick and answer. The rows are then
display-only: the component still draws buttons, but they do nothing, and
the app returns the engine's error.

**Ids.** The library key is the checklist id. New ids come from `next_id`, the
counter every other minted id uses:

- a template is `checklist-{n}`, with root task `checklist-{n}/root`;
- a run is `run-{n}`, and its items are derived `"run-{n}/{template task}"`
  (C1);
- an item added to a template gets its id from `next_task_id`.

## Tests (C3a)

- The existing switcher test's label arrays and lengths gain Checklists.
- `REQUIRED_PROPS`, `every_declared_event_is_accepted`, and the contract's
  view mapping all gain the new slots and events.
- Walk-through tests:
  - create → add items → start → tick all → complete, in which Complete is
    offered only at the end;
  - a decision run, where branches appear only after an answer and a repeated
    answer clears it;
  - abandon, after which the run is read-only;
  - delete, which clears the selection;
  - restore round-trips the selection;
  - an old snapshot without the new fields restores.
- `package_compiles` checks that the new slots are declared.

## Deferred

- The web host (C3b), and decision authoring (C3c).
- Reordering and renaming items in the view; per-item notes.
- Showing run items in search.
- Retiring `checklist-app` and `release-checklist.yml`, which happens after
  C3b ships.
