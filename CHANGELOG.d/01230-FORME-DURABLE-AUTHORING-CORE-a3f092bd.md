### Forme durable authoring core

- Define FM09 and split the broad authoring-shell milestone into independently
  reviewable data, editor, preview, publish, and desktop product boundaries.
- Add a bounded closed authoring-project codec over Content IR with canonical
  persistence and hostile-input validation.
- Add immutable semantic edits, atomic compare-and-swap autosave, serialized
  concurrent commands, and bounded undo/redo that survives restarts.
- Bound hostile input before allocation, consume descriptor snapshots without
  invoking getters, and define cancellation and indeterminate adapter behavior
  at the atomic storage commit point.
