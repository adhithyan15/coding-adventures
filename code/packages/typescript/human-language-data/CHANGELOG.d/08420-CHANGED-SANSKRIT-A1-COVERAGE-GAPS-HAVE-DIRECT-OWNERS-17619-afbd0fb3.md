### Changed — Sanskrit A1 coverage gaps have direct owners (#17619)

- Replace the shared hand-edited 164/157/7 census, category counters, and
  cumulative history with exact totals derived from the direct point inventory
  and seven canonical direct gap owners.
- Fail closed when a gap owner disappears, names an unknown point, or disagrees
  with measured unmapped or partial evidence; keep the ordinal and
  relative-position evidence in independent hashed case owners.
- Remove the migration-only whole-inventory digest so a direct point's probe can
  change without requiring an unrelated shared test edit.
