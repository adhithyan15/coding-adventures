# Changelog

## Unreleased

### Added — `ChecklistRun` (0.1.0, C2 of #14018)

- **What it shows:** working through one checklist run. A title (heading),
  progress, the visible rows, Yes/No on each question, and label-gated
  Complete and Abandon actions.
- **Rows** are task-core's `checklist_run` rows as positional
  `list<list<text>>` with truthy markers only, so the layout never compares
  strings.
- **Check items are toggle buttons** that report the kernel `selected` state
  (UI86) and carry their row index (UI37). A checkbox's toggle can't say which
  item it was.
- **Tests:**
  - Pinned: the interface, the heading, selected state on all six row-button
    branches, truthy-only conditions, gated finishing actions, and childless
    buttons (#15921).
  - Builds on all eight backends.
  - The native gate passes with zero capability degradations. Two
    pre-existing Flutter style gaps are pinned (#12022).
- **MosaicBook:** three stories: just started, a question answered with its
  branch revealed, and complete.
