# Changelog

## Unreleased

### Changed — a check item is a real `HostCheckbox`

- A check item is the platform's own checkbox, labelled with the item's name,
  instead of a toggle `HostButton` beside a "☐"/"☑" text glyph. It has the
  native checkbox role, keyboard and focus behaviour, and the glyph's font
  fallback (small and top-aligned on Compose) is gone.
- `onToggle ( index : number )` is unchanged: UI29-2 §2.1.1 now gives a
  checkbox in a `For` the row index, the rule `HostButton` already followed.
- Parts: `checklist-item` / `checklist-item-done` style the checkbox label
  (the done item is muted). `checklist-box`, `checklist-box-done`,
  `checklist-item-button` and `checklist-item-button-done` are removed. Label
  size is the platform's own: Qt's `CheckBox` does not lower `font-size`, and
  the native gate allows no drop.
- Rendered in Trestle on Compose, both themes: ticking an item through the
  checkbox flips exactly that row ("1 of 1 done") and mutes it; unticking
  restores it.

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
