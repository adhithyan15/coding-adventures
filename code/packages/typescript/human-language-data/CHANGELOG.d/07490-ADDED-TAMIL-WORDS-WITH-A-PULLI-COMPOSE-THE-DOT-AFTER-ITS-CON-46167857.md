### Added — Tamil words with a pulli compose, the dot after its consonant

- `src/figure-targets.ts`: `WRITTEN_SIGN_SIDES.tamil` gains the pulli ்
  (U+0BCD) as "after", so க் is written க, then the dot, and வணக்கம் is
  வ, ண, க, ், க, ம, ். The row is cited in the pulli's mark record
  (`compositionOrder` and `compositionSource`: Varai's recorded drawings of
  the 18 consonants with pulli draw the body first and the dot second, one
  writer, confidence medium), and the existing test holds the table to it.
- **Fused across the pulli.** A pulli ends a grapheme, so `writtenPiecesOf`
  could not see letters the font joins across it. The new
  `FUSED_LETTER_SEQUENCE_SOURCES` (built into `hasFusedLetterSequence`)
  lists the runs the bundled Noto Sans Tamil 2.004 prints as one glyph:
  க்ஷ (GSUB `akhn`, lookup 2) and ஸ்ரீ / ஶ்ரீ (GSUB `abvs`, lookup 1). A word
  containing one is refused. Every other consonant + pulli prints as the
  unchanged consonant with the unchanged dot, so it is drawn in parts.
- **Unlocked: 17 Tamil lessons** (Tamil filmstrip target count 48 -> 65):
  TA-S08-pulli and TA-W03-pulli-vanakkam (the sign alone), and the words in
  TA-W03-write-vanakkam, W04-i-sign-write-nandri, W05-write-aam,
  W06-write-illai, W08-read-en, W09-read-peyar, W10-read-naan,
  W11-read-niingal, W16-read-tamizh, W18-read-uur, W23-read-sattai,
  W27-read-payam, W31-read-aanaal, W32-read-sol and W33-read-een. Words with
  ு or ூ, or the fused டி, stay refused (சொல்லுங்கள், எப்படி, வண்டி and
  others).
- Regenerated: the filmstrip ledger's Tamil owner (one entry added), 17
  SVGs, the Tamil figure-hash owner, 16 Tamil chapters (one
  `\hlblockfigure` each) and the pulli's script-owner evidence. Narration,
  modality and lesson prose do not change.
- Tests: the written-order case covers the pulli, three words with it, the
  fused runs and the bundled font's version; the real-corpus case pins the
  new single, sequence and word strips; the sequence-strips case uses பேசு as
  its refused word; the pulli's inventory evidence pins its record.
