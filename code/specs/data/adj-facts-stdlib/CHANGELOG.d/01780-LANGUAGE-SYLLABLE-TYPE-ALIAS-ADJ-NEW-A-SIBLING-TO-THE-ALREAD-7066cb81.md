- `language/syllable-type-alias.adj` (new) — a sibling to the already-shipped `silent-e-word.adj`
  (`silent_e_word(word, syllable_type)`, wake/whale/while/yoke/yore/rude/hare → all
  vce_long_vowel). That table's own `source` field already quotes, verbatim, the Reading Rockets
  sentence in full — "Also known as \"magic e\" syllable patterns, VCe syllables contain long
  vowels spelled with a single letter, followed by a single consonant, and a silent e." — but the
  word/syllable_type schema had room only for WHICH words follow the pattern, not the sentence's
  own opening alias clause naming the pattern's alternate name. New
  `syllable_type_alias(syllable_type, alias)` table decodes that clause as its own row:
  vce_long_vowel → magic_e. Since the parent tables only ONE syllable type across all seven rows,
  the abstention target instead comes from the parent's own "Six Syllable Types" source article —
  honest abstention on closed_syllable, a real syllable type that article covers but
  silent-e-word.adj (and therefore this sibling) does not table. New e2e test file
  `facts_syllabletypealias_e2e.rs` (3 tests: forward recall with citation, backward recall, honest
  abstention). No manifest objective, matching `silent-e-word.adj`'s own precedent. Sixth slice
  from the language/ domain cleanup sweep.
