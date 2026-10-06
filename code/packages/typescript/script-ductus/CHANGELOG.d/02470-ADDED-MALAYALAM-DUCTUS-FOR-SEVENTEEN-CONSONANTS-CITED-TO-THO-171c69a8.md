### Added Malayalam ductus for seventeen consonants cited to Thooval

- Seventeen base consonants enter `src/strokes/malayalam.ts`, appended after
  ഴ so no existing key moves: ന മ സ ര ത ഷ പ വ ണ ട ദ ഹ ഗ റ ല ശ ബ. Their
  bare-consonant rows in `malayalam.json` (role `syllable`) now carry a
  `strokeOrderSource`, read through the existing `malayalamAlphabetSource`.
- **Order, start, turns and end** come from the formation images of SPACE
  Kerala's *Thooval* Malayalam alphabet learning tool
  (`github.com/spacekerala/Thooval`, commit `87143b5`): each `data/<SLUG>.png`
  marks the start in green, every turn where the pen runs back along its own
  ink in blue, and the end in red, with arrows between. Thooval is GPL-3.0, so
  it is cited for those facts only; no image, path or template point is
  copied. Each record links the image at the pinned commit.
- **Pen lifts: none.** Thooval asks the learner to keep the pen down to the
  end of every letter, so its single run is not evidence by itself. Two
  recorded sources that could have shown a lift show none: Santhosh
  Thottingal's *hand* reference curves (MIT), which store several strokes
  where a glyph has them, record every one of these letters as one stroke;
  the *grahyam* online-handwriting samples (no licence; counts only) keep no
  pen-up marker, but no unique sample (31 to 107 per letter) jumps between
  consecutive points the way a lift would. Starts and ends
  agree in all three sources except where each record says otherwise (മ's
  start and end, ഷ's end). Omniglot's non-native copyists most often used two
  strokes for മ; the record says so.
- **Fitted, not traced.** Every path follows the skeleton of the bundled Noto
  Sans Malayalam outline between Thooval's turning points; where Thooval turns
  back at a stem's foot (ന സ ണ ബ) or at ദ's middle tip, the path retraces that
  ink. Default tolerances, no overrides: `fractionOnInk` 1.0000 on every
  stroke, join gaps 0, no untraced ink (456 to 1,142 sample points per glyph).
  Every caption fits its printed panel in at most two lines, checked on the
  rendered SVGs.
- **Held back:** ക and യ (the sources disagree on the start), ഏ (Thooval's
  file name is uncertain and grahyam has 6 samples) and ം (no source).
- Tests: `tests/strokes/malayalam.test.ts` pins each letter's labels, single
  run, joins and Thooval URL; `tests/ductusview/malayalam.test.ts` pins each
  strip's movement count and "one unbroken stroke" summary.
  `tests/stroke-ownership.test.ts`: keys 534 -> 551, Malayalam 14 -> 31, new
  ordered key hash and non-Tamil data hash, measured after the captions were
  settled. The filmstrip ledger's Malayalam owner gains the seventeen glyphs.
