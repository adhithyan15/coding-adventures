- `language/morse-code-standard.adj` (new) — a sibling to the already-shipped `morse-code.adj`
  (`morse_code(letter, pattern)`, the 26 letter→dot/dash mappings). That table's own `source` field already
  quotes, verbatim, a Wikipedia sentence naming the international standard that specifies the code — "the
  current international standard, International Morse Code Recommendation, ITU-R M.1677-1" — a fact the
  letter/pattern schema had no room for. New `morse_code_standard(code_system, standard_id)` table decodes
  that clause as its own row: international_morse_code → itu_r_m_1677_1. Honest abstention on
  american_morse_code, a real but distinct historical Morse variant the cited span does not name. New e2e
  test file `facts_morsecodestandard_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `morse-code.adj`'s own precedent. First slice from a
  targeted sibling-table sweep of the language/ (literacy) domain — this domain had already been swept
  repeatedly for this pattern, but 7 small untabled leftover facts remained; this is the first of them.
