# mosaic-pkg-checklist

`ChecklistRun` shows what someone sees while working through one run of a
checklist: the visible items, ticked or not, the yes/no questions with their
answers, the progress, and the actions that finish the run. C2 of
[#14018](https://github.com/adhithyan15/coding-adventures/issues/14018), which
folds the standalone Checklist app into Trestle. The design is in
[`code/specs/mosaic-pkg-checklist.md`](../../../specs/mosaic-pkg-checklist.md).

## Where it fits

```
task-core checklists (C1)   templates, runs, the visible-rows projection
  └ ChecklistRun (C2)       ← this package: the run, rendered
      └ Trestle Checklists (C3)   the surface that mounts it
```

The rows are exactly `task-core`'s `checklist_run(..).rows`, which holds only
the visible items. So the component never needs to know about branches: an
unanswered question simply has nothing under it.

## Interface

```
slot title, progress-label, yes-label, no-label, complete-label, abandon-label : text ;
slot rows : list<list<text>> ;   // [key, indent, name, decision?, ticked-or-yes, no] — "1" or ""
emit onToggle ( index : number ) ;  emit onAnswerYes ( index : number ) ;
emit onAnswerNo ( index : number ) ;  emit onComplete ;  emit onAbandon ;
```

A check item is a toggle button beside a ☐/☑ mark, not a checkbox. Inside a
`For`, a button's click carries its row index on every backend, but a
checkbox's toggle carries only the new boolean.

## Testing

```sh
cargo test   # interface + layout promises, builds on all 8 backends, native gate
```
