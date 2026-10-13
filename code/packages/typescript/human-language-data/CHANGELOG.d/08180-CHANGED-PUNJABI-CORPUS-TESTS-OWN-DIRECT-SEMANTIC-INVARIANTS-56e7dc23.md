### Changed — Punjabi corpus tests own direct semantic invariants (#17417)

- Derive the Punjabi session-map range from its row owners while retaining
  exact equality with canonical lesson ids and chapters. Appending one lesson
  no longer requires a separate corpus-wide length edit.
- Let the budget helper derive the generated lesson count while retaining the
  stable idiom, sense, culture-claim, and unit-prefix requirements.
- Scope the Chapter 3 continuity assertion to Chapter 3 atoms, and pin the
  first stable owner of each pre-A1 writing stage instead of serializing every
  later repetition into a shared expected array.
- Add a source-level regression that rejects the retired aggregate shapes so
  independent Punjabi lesson owners cannot quietly regain shared counters.
