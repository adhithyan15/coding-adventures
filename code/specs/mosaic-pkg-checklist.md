# mosaic-pkg-checklist — working through a checklist run (C2)

Issue: [#14018](https://github.com/adhithyan15/coding-adventures/issues/14018) (C2) ·
Engine: [`task-app-checklists-v1.md`](task-app-checklists-v1.md) (C1, merged) ·
Program: [#14415](https://github.com/adhithyan15/coding-adventures/issues/14415) §2

## What it is

One component, `ChecklistRun`: what someone sees while working through one run
of a checklist — the visible items, ticked or not; the yes/no questions, with
their answers; progress; and the actions that finish the run. C1 put the model in
task-core (`checklist_run` returns exactly the visible rows); this is its view.
C3 mounts it as Trestle's Checklists surface.

Its own package, like `mosaic-pkg-notes`, so Trestle composes it and nothing
else has to carry it. Authoring a template (both branches, add/remove items) is a
different screen and a later component.

## Interface

```
component ChecklistRun {
  slot title          : text ;              // the run's name (a heading)
  slot progress-label : text ;              // "3 of 5 done" — the host formats it
  slot rows           : list<list<text>> ;  // see below
  slot yes-label      : text ;              // "Yes" — localisable
  slot no-label       : text ;              // "No"
  slot complete-label : text ;              // "" = not offered (e.g. not yet complete)
  slot abandon-label  : text ;              // "" = not offered
  emit onToggle    ( index : number ) ;     // tick / untick a check item
  emit onAnswerYes ( index : number ) ;
  emit onAnswerNo  ( index : number ) ;
  emit onComplete ;
  emit onAbandon ;
}
```

### Rows

Positional `list<list<text>>`, like `RecordList` — and, like `ProjectNav`, using
only **truthy markers**, so the layout needs no string comparison:

| field | check item | decision |
| --- | --- | --- |
| `row[0]` key | task id | task id |
| `row[1]` indent | leading glyphs for depth (`""` at depth 0) | same |
| `row[2]` name | the item | the question |
| `row[3]` decision marker | `""` | `"1"` |
| `row[4]` | `"1"` when ticked | `"1"` when answered yes |
| `row[5]` | `""` | `"1"` when answered no |

The rows are exactly task-core's `checklist_run(..).rows` — the **visible**
items, so an unanswered question shows no branch and the component never has to
know about branches at all.

### Why buttons, not checkboxes

A check item is a `HostButton` beside a ☐/☑ mark, reporting `selected` (UI86)
when ticked, not a `HostCheckbox`. Inside a `For`, a `HostButton`'s click carries
its row index (UI37) on every backend; a checkbox's toggle carries only the new
boolean, which cannot say *which* item. A toggle button whose pressed state is
the tick is also the accessible reading of "mark this item done".

Each answer is two buttons, Yes and No, the chosen one `selected` — a two-option
segmented choice, which is what a decision is. Completing is gated on the host's
`complete-label`: task-core refuses to complete an incomplete run, and the host
offers the button only when `progress.complete`.

## Styles

Only properties every native backend lowers; the package's native gate allows no
capability degradation and pins style drops in both directions.

## Deferred

Template authoring (both branches visible, add/remove/reorder items); per-item
notes; a run library view (C3 composes `RecordList` over `checklists()` for that).
