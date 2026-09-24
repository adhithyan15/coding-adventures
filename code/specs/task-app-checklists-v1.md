# Trestle checklists v1 — templates and runs in task-core (C1)

Issue: [#14018](https://github.com/adhithyan15/coding-adventures/issues/14018) (C1) ·
Program: [#14415](https://github.com/adhithyan15/coding-adventures/issues/14415) §2 ("Checklist is a feature, not a product") ·
Builds on: `task-app-data-model.md` "Decision-tree support"

## Why

The standalone Checklist app (`code/programs/typescript/checklist-app`) is being
folded into Trestle. What it has that Trestle lacks is not the decision node —
task-core already models one (`Task.decision`, and `checklist()` already hides
the unchosen branch) — but three things around it:

1. **A checklist as a thing.** A named, reusable list — "Pre-flight", "Release".
2. **Template vs. run.** You author a template once and run it many times; each
   run is independent, and answering a question in a run never edits the
   template. Today an answer is written onto the task itself, so the "template"
   *is* the run.
3. **Run semantics.** A run is in progress, completed, or abandoned; it has
   progress; and it can only be completed when every visible item is done.

C1 adds exactly these to the engine (task-core + the task-wasm ABI). The Trestle
surface — a Checklists view beside List/Board/Sheet/Calendar/Notes — is C3; the
components are C2.

## Model

A checklist names a **subtree**. Its items stay ordinary `Task`s and its
decision nodes stay `Task.decision`, so every existing tool (notes, labels,
priorities, search) keeps working on them.

```rust
pub struct Checklist {
    pub id: ChecklistId,
    pub name: String,
    pub description: String,
    pub root: TaskId,               // the items are root's subtree, incl. decision branches
    pub created_at: u64,
    pub run: Option<ChecklistRun>,  // None = a template; Some = a run
}
pub struct ChecklistRun {
    pub template: ChecklistId,      // may dangle after the template is deleted
    pub status: RunStatus,          // InProgress | Completed | Abandoned
    pub finished_at: Option<u64>,
}
// ProjectState.checklists: BTreeMap<ChecklistId, Checklist>   (#[serde(default)])
```

`#[serde(default)]` keeps every pre-C1 snapshot loading.

### The decision invariant

A decision's branch children are its outline children, and nothing else is:

- every id in `yes_children ∪ no_children` has `parent == Some(decision)`;
- no id is in both branches, or listed twice, or is the decision itself;
- the decision has no outline child outside the two branches.

Before C1 none of this was checked, so a branch child parented elsewhere showed
whatever the answer, and an outline child outside both branches never showed.
`set_decision` now enforces it (reparenting the listed children under the
decision, rejecting cycles), and `delete_task` removes a deleted id from every
decision's branch lists — previously it left dangling references.

## Operations

Every op validates before it writes; a rejected op leaves the state unchanged.

| op | effect |
| --- | --- |
| `create_checklist_template(id, root, name, description, now)` | creates the root as a Summary task and registers the template; items are then added with `create_task(…, Some(parent))` and `set_decision` |
| `instantiate_checklist(template, run, now)` | deep-copies the template subtree into a new run: checks unticked, answers cleared, status `InProgress` |
| `clear_decision_answer(task)` | back to unanswered (the TS app's `answer: null`) |
| `complete_checklist_run(run, now)` | `Completed`, stamped — **rejected unless the run is complete** (below) |
| `abandon_checklist_run(run, now)` | `Abandoned`, stamped |
| `delete_checklist(id)` | removes the checklist and its whole subtree; runs of a deleted template survive |

**Run ids are derived, not minted.** The core never generates ids (the host
does). `instantiate_checklist` names each copied task `"{run}/{template task}"`,
which is deterministic and needs no extra ids from the host; any collision is
rejected before anything is written.

**Templates are not answered.** `answer_decision` and `clear_decision_answer`
on a template item are rejected, as is `set_decision` with an answer; answers
belong to runs. A run's items can be ticked and answered but a finished
(completed or abandoned) run is read-only.

**Boundaries.** Every op that could move work into, out of, or within a
checklist respects it (added after the pre-push security review found each of
these reachable):

- A checklist's **root never moves** (`reparent`, `move_task`) and is never
  deleted on its own. Reparenting a run's root under its own template doubled
  the template on every instantiate.
- **Nothing crosses a checklist boundary.** `reparent` and `set_decision`'s
  reparenting refuse to move an item between checklists, or between a
  checklist and the project. Otherwise an answered decision could be moved
  into a template. `move_task` refuses checklist items altogether.
- **A finished run is frozen:** no `create_task` under it, and no
  `delete_task`, `reparent`, `set_decision` (not even clearing a decision) or
  `set_status` on its items. `set_status` counts as ticking, because a done
  status sets `completed`. What is frozen is the *record of the work*: ticks,
  answers and structure. Text and display fields stay editable (names, notes,
  order, labels, priority), so a typo in a finished run can still be fixed.
- A run id is at most 256 bytes (`MAX_RUN_ID_BYTES`), because every copied item
  repeats it.
- `ensure_default_workflow` does not stamp checklist items, since stamping would
  tick template items.
- A template holds at most 10,000 items (`MAX_CHECKLIST_ITEMS`), so one
  `instantiate_checklist` cannot allocate without limit. A template whose root
  is missing cannot be instantiated.
- On a malformed snapshot where two checklists share a root, the most
  restrictive owner governs editing: template, then finished run, then live run.

**Hidden branches keep their state.** Switching an answer from yes to no hides
the yes branch without unticking it, exactly as the TS app did, so switching
back restores the work.

## Projections

- **`checklists()`** — every template and run for the library: name, template
  link, status, progress, timestamps.
- **`checklist_run(run)`** — the visible rows (reusing `ChecklistRow`), status,
  progress and duration.
- **`checklist_outline(template)`** — every row with both branches shown and
  labelled, for editing a template.

**Progress** counts **visible** items only (an unanswered decision's branches
are invisible), exactly as the TS app's `computeStats`: `total`/`checked` over
check items, `decisions`/`answered` over decisions, `percent` = checked/total
(0 when there are no checks), and

```
complete = (total == 0 || checked == total) && answered == decisions
```

— the TS Complete button's rule, now enforced by the engine.

**Checklist items stay out of the general views.** Template and run items are
excluded from List/Sheet/Calendar (`view::select`), `todos()`, `kanban()`,
`flowchart()` and the legacy project-wide `checklist()`; otherwise a template's
items would appear as open todos. Surfacing *run* items in search is a later
decision (program §2's "one search").

## Out of scope for C1

The Checklists surface in Trestle (C3), the components (C2), amending
`checklist-app.md`, and retiring `release-checklist.yml` — which happens only
after Trestle ships the surface.
