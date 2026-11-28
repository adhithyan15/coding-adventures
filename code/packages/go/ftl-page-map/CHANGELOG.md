# Changelog

## Unreleased

- Add a dependency-free logical-to-physical page map with lowest-free
  reservations, ready publication, fresh and spent abort dispositions, typed
  refusals, atomic transitions and detached snapshots.
- Add model-first bounded fixture replay, fixed-point enumeration for the
  smallest geometries, seeded and fuzzed histories, race checks, coverage and
  three disposable mutation controls.
- Add per-call allocation fences and atomic release of eligible erased extents,
  with a separate model-first shared vector, exhaustive small-state closure,
  bounded fixture and history fuzzing, and three reclamation mutation controls.
- Reject null integer items, incomplete reservations, and case-insensitive JSON
  aliases in the original page-map test fixture adapter before model replay.
- Add `InvalidateCurrent` to remove one exact mapping while retaining its
  physical slot as stale until an erased extent is explicitly released.
- Add `RecordUnownedProgram` to mark one free physical page stale without
  changing logical ownership, with exact refusal ordering and model-checked
  allocation skipping and post-erase reuse.
