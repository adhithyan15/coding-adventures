### Changed — Spanish mock audit uses bounded reads

- Build the pre-A1, A1, and A2 book-bounded audit from Spanish lessons,
  curriculum membership, and the common spine rather than parsing every
  unrelated language on every report and `--check` call (#18749).
- Keep the exact taught-headword conversion, documented credits, canonical
  report bytes, and linked-track rejection. Add regressions that fail if the
  whole-corpus loader returns to this Spanish-only path.
- On Windows, the focused audit file fell from a 208-second import with three
  60-second `--check` timeouts in the full diagnostic to a 9.6-second import
  and 50/50 passing tests in a focused run. This does not claim that the
  full package suite or #12732 is fixed.
