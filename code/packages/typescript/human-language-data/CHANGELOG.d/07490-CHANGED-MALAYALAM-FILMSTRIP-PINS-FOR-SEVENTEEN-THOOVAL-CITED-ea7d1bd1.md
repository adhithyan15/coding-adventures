### Changed — Malayalam filmstrip pins for seventeen Thooval-cited consonants

- `data/scripts/malayalam.json`: the bare-consonant rows of ന മ സ ര ത ഷ പ വ ണ
  ട ദ ഹ ഗ റ ല ശ ബ now carry a stroke order (one sentence per movement), a
  stroke-order note, `penLifts: 0` and a `strokeOrderSource` citing SPACE
  Kerala's Thooval formation arrows (GPL-3.0, facts only), with
  santhoshtr/hand (MIT) and grahyam (counts only) as the evidence for no lift.
  ക, യ, ഏ and ം stay unverified.
- `tests/filmstrip-target-counts/malayalam.json` 14 -> 35: the seventeen letter
  lessons (ML-S05, S08, S109, S110, S112, S113, S114, S117, S121, S123, S124,
  S127, S132, S136, S146, W113-sha, W114-ba) and the four ML-W01 നമ writing
  lessons (trace, guided copy, delayed copy, dictation) now print a filmstrip.
- `tests/figure-targets/the-real-corpus-*.case.ts`: the word-strip pin gains
  ML-W01-na-ma-guided-copy, -delayed-copy and -dictation (headword നമ, drawn
  letter by letter because Malayalam letters stand apart); the trace lesson's
  "ന മ" is a list and stays off that pin.
- `tests/script-inventories/malayalam.evidence.ts` pins each row's lifts,
  movement count, URL, citation and variation wording, and pins ക and യ as
  still unverified.
- Regenerated: the Malayalam filmstrip geometry, 21 filmstrip SVGs, the
  Malayalam figure hashes and 14 Malayalam book chapters. Lesson prose,
  narration and modality do not change.
