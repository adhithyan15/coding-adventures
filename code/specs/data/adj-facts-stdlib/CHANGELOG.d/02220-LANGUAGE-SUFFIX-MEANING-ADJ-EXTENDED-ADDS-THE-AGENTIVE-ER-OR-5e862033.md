- `language/suffix-meaning.adj` (extended) — adds the AGENTIVE `-er`/`-or` sense ("one who;
  person connected with", as in "teacher"/"professor") flagged as a future extension candidate
  when this table originally shipped: `row (_er_agentive, one_who_or_person_connected_with)`,
  `row (_or, one_who_or_person_connected_with)`, decoded from the SAME already-cited Reading
  Rockets "Common Suffixes" chart's own bundled "-er, -or" row — zero new sourcing, the PDF was
  already curl-fetched and read byte-for-byte for the original round. Resolves the real
  heteronym-in-spelling risk that original round's header flagged but deliberately left open:
  `-er` appears TWICE in the source chart, once as this table's own already-excluded COMPARATIVE
  sense ("more", as in "taller") and once as this newly-shipped AGENTIVE sense — applying the
  SAME `ow`/`ow_long_o`-style disambiguated-atom discipline `long-vowel-team-sound.adj`
  established, the agentive sense ships as `_er_agentive` rather than a bare `_er`, so a future
  round can still add a comparative `-er` row without ever colliding with this one. `-or` carries
  no known collision risk (not used elsewhere in this table or any sibling table), so it ships as
  the plain atom `_or`. `_er_agentive` and `_or` share one meaning atom, the SAME bundled-row
  shape `_able`/`_ible` already established for the chart's own "-able, -ible" row — a reverse
  query on `one_who_or_person_connected_with` genuinely binds BOTH spellings. Empirically verified
  both new atoms (forward and reverse) against the real built `adj-lang-cli` binary in a scratch
  table before writing the shipped file. No new manifest objective (same library, same objective
  `adj.literacy.3to5.suffix_meaning`). Extended `facts_suffixmeaning_e2e.rs` from 5 to 7 tests (a
  new forward recall on the disambiguated `_er_agentive` atom, and a reverse recall binding both
  `_er_agentive` and `_or`). `_ic` remains an honest abstention (unchanged).

