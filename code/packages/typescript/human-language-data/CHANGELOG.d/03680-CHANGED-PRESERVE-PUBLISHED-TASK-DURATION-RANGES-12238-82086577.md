### Changed - preserve published task-duration ranges (#12238)

- Accept either an exact positive minute count or an exact positive
  `{ minimum, maximum }` range for task-shape administrations and sections.
- Compare normalized duration ranges when checking written totals and speaking
  duration, while keeping existing exact-number inventories backward compatible.
- Reject reversed and mismatched ranges rather than rounding an awarding body's
  published range to one invented minute.

