- `language/morse-code-origin.adj` (new) — a sibling to the already-shipped `morse-code.adj` and
  `morse-code-standard.adj`. The SAME already-quoted Wikipedia sentence also names who proposed the code
  International Morse code was derived from, and when — "a much-improved proposal by Friedrich Gerke in
  1848" — a fact neither sibling's schema had room for. New `morse_code_origin(code_system, originator,
  year)` table decodes that clause as its own row: international_morse_code → friedrich_gerke, 1848. A
  THREE-column table, since originator and year belong together as one origin event rather than two
  separate sibling tables. Honest abstention on american_morse_code, the same target
  `morse-code-standard.adj` already uses. New e2e test file `facts_morsecodeorigin_e2e.rs` (3 tests:
  forward recall with citation, backward recall, honest abstention). No manifest objective, matching
  `morse-code.adj`'s own precedent. Second slice from the language/ domain cleanup sweep.
