### Added — Devanagari signs taught alone print filmstrips; Devanagari words stay refused

- **Bare-sign lessons only.** Eight Devanagari mark records (ु ू े ं ़ ् ृ ँ)
  gain `components`, `strokeOrder`, `strokeOrderNote`, `penLifts` and a
  `strokeOrderSource` citing native writers' pen traces in HP Labs India's
  LipiTk Devanagari recognizer, with counts and shares in each `variation`.
  A lesson whose headword is one of those signs by itself becomes a
  one-glyph filmstrip target through the existing `writingLetterOf` path.
- **No written-order claim.** The writers wrote each sign alone, so no record
  gains a `compositionOrder` (the nukta keeps its earlier Unicode-cited
  carrier-first convention), and `WRITTEN_SIGN_SIDES` gets no Devanagari row:
  a sign on a consonant (कि, कु) and every Devanagari word with a sign stay
  refused. No source change in this package.
- **Evidence.** A new case, `devanagari-signs-drawn-alone`, holds that no row
  exists, that a bare sign is one glyph and a consonant-plus-sign is refused,
  and that exactly the eight records cite a ductus and none claims a written
  order. A new `script-inventories/devanagari-marks.evidence.ts` pins every
  Devanagari mark record (digest, `strokeOrder`, `penLifts`, recognizer
  class, source wording) and says why the other seven claim nothing.
- **32 lessons gained a filmstrip:** Hindi 44 → 54, Marathi 42 → 49,
  Sanskrit 39 → 48, Marwadi 33 → 39 (target-count pins). The real-corpus
  case pins all 32 and the sign lessons still refused (ा ि ी ो ौ ै ः, and
  HI-W03-preposed-i, which has no Writing or Script block). Regenerated: 32
  SVGs, the four figure-hash owners and 22 book chapters (one
  `\hlblockfigure` each). Narration and modality do not change.
