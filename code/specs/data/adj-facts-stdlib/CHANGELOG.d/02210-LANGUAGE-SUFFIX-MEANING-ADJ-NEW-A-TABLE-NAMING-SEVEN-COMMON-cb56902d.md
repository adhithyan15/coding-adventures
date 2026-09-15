- `language/suffix-meaning.adj` (new) — a `table` naming seven common derivational suffixes and
  what each actually means, quoted verbatim from Reading Rockets' "Common Suffixes" chart
  (reproduced with permission from Corwin Press): `suffix_meaning(suffix, meaning)`, `_ful` →
  `full_of`, `_less` → `without`, `_able`/`_ible` → `is_or_can_be`, `_ness` →
  `state_of_or_condition_of`, `_en` → `made_of`, `_y` → `characterized_by`. The sibling to
  `prefix-meaning.adj`, mirroring its trailing-underscore atom-label convention with a
  LEADING-underscore label instead (`_ful` stands for "-ful", attaching to the END of a word) —
  empirically verified the `adj-lang` lexer's `IDENT` token pattern (`[a-z_][a-z0-9_]*`) accepts
  a leading underscore exactly as it accepts a trailing one, against the real built CLI in a
  scratch table before writing this file. A genuinely DIFFERENT source family from Grammarly,
  which was tried and rejected twice before for this exact angle (its suffix prose bundles
  multiple senses per paragraph rather than one clean per-suffix meaning, unlike its own prefix
  article) — Reading Rockets is an ALREADY-vetted source family in this stdlib (17 prior
  citations, all phonics/phonemic-awareness content), but this is the first citation of its
  "Common Suffixes" chart specifically, confirmed via a full-tree grep that no existing table
  names "suffix" as a row value. `_able`/`_ible` deliberately ship as two rows sharing one
  meaning atom, since the source's own chart bundles "-able, -ible" into one row — the same
  many-keys-to-one-value shape `diphthong-sound.adj`'s `oi`/`oy` → `oi_sound` pair already
  established. Deliberately excludes the chart's own purely-inflectional rows (`-ed`, `-s`/`-es`,
  comparative `-er`, superlative `-est`), each already covered by a sibling table in this stdlib
  (`past-tense-ed-sound.adj`, `plural-s-sound.adj`, `superlative-adjective-rule.adj`). Flags, but
  does not yet need to resolve, a real heteronym-in-spelling risk the source's own chart
  surfaces: the excluded comparative `-er` ("more") and a real, not-yet-shipped AGENTIVE `-er`
  ("one who") are two genuinely different suffixes sharing one spelling — the same kind of
  collision `long-vowel-team-sound.adj` resolved for `ow`/`ow_long_o` — logged in this file's own
  header for whichever future round adds the agentive sense as a new row. curl-fetched the raw
  PDF directly and read it byte-for-byte before writing this file. Honest abstention on `_ic`
  ("having characteristics of", a real suffix the same chart covers, but a near-duplicate of the
  chart's own separate, also-unshipped `-al`/`-ial` definition, avoiding the near-synonym
  pile-up `prefix-meaning.adj`'s own design note cautions against). New
  `suffix-meaning.query.adj` and `facts_suffixmeaning_e2e.rs` (5 tests: forward recall, reverse
  recall, reverse recall binding BOTH spellings that share one meaning, a second forward recall
  on a distinct semantic category, and honest abstention); new manifest objective
  `adj.literacy.3to5.suffix_meaning`. Full verification pipeline green (see PR).

