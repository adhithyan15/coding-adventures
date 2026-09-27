### Elixir build-tool portable-glob hardening

- Replaced recursive grapheme matching with bounded Unicode-scalar rolling-row
  dynamic programs and a linear character-class parser.
- Added strict Python-fnmatch character classes, stable typed failures, and
  complete declared-source-list validation before selection and hashing
  short-circuits.
- Covered adversarial stars, globstars, unmatched brackets, supplementary
  scalars, decomposed Unicode, and invalid later patterns without adding host
  authority.
