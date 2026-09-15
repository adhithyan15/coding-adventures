- `language/greek-alphabet-standardization.adj` (new) — a sibling to the already-shipped
  `greek-alphabet.adj` (`greek_letter_position(letter, position)`, the 24 letter→position mappings).
  That table's own `source` field already quotes, verbatim, a Wikipedia sentence naming WHEN the
  Euclidean alphabet the letter-position table is built from became standard — "by the end of the 4th
  century BC, the Ionic-based Euclidean alphabet, with 24 letters, ordered from alpha to omega, had
  become standard" — a fact the letter/position schema had no room for. New
  `greek_alphabet_standardization(alphabet_name, standardized_by_period)` table decodes that clause as
  its own row: euclidean_alphabet → fourth_century_bc. (Note: the ADJ atom is spelled `fourth_century_bc`,
  not `4th_century_bc` — ADJ atoms must lex as identifiers and cannot start with a digit.) Honest
  abstention on attic_alphabet, a real but distinct Greek alphabet variant the cited span does not name.
  New e2e test file `facts_greekalphabetstandardization_e2e.rs` (3 tests: forward recall with citation,
  backward recall, honest abstention). No manifest objective, matching `greek-alphabet.adj`'s own
  precedent. Third slice from the language/ domain cleanup sweep.
