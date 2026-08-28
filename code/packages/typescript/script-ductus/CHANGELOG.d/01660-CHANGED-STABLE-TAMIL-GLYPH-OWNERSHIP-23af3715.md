### Changed — stable Tamil glyph ownership

- Move every Tamil Ductus record, stroke-evidence suite, and filmstrip-evidence
  suite into a matching ASCII code-point owner while keeping the Tamil roots
  assembly-only.
- Preserve the exact global registry order and runtime data, including sourced
  short `ஒ`, while moving mutable data proof beside each owned glyph.
- Discover and reject missing, duplicate, or mismatched Tamil owners so edits to
  different existing glyphs remain merge-conflict independent.
