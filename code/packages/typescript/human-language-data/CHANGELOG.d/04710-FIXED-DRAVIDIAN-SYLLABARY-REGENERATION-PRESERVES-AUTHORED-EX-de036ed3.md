### Fixed - Dravidian syllabary regeneration preserves authored extensions

- Make `generate_syllabary.py` merge the committed Telugu, Kannada, and
  Malayalam extensions back onto their freshly generated Unicode identities.
- Preserve sourced rows, core-external letters, marks, final consonants, future
  evidence fields, and established top-level ordering while failing closed on
  malformed or duplicate glyph identities.
- Add six focused Python regressions, including corpus-wide semantic
  idempotence across all three generated scripts, and run them in the
  human-language book workflow before any generated artifact reaches readers.

