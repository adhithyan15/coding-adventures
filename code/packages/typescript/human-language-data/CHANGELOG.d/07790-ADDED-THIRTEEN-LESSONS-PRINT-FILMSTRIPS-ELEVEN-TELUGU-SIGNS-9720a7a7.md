### Added — thirteen lessons print filmstrips: eleven Telugu signs, Bengali ং and Tamil ஸ

- **13 lessons gain a strip** from the new script-ductus entries, each cited
  to native writers' pen traces in HP Labs India's LipiTk 4.0 recognizers
  (MIT model; counts and shares only): Telugu TE-S02 (ు), TE-S04 (ం), TE-S05
  (్), TE-S07 (ా), TE-S08 (ి), TE-S115 (ె), TE-S119 (ీ), TE-S120 (ే),
  TE-S134 (ూ), TE-S153 (ో) and TE-S154 (ొ); Bengali BN-W41-anusvar (ং); Tamil
  TA-S129-letter-sa (ஸ). Each Telugu sign is drawn alone: Telugu has no
  `WRITTEN_SIGN_SIDES` row, so no consonant + sign headword and no Telugu word
  is composed from them. 806 -> 819 of 1,367 writing lessons have a strip.
- **Inventory.** `telugu.json` gains nine vowel-sign mark records (ా ి ీ ు ూ ె
  ే ొ ో), and its ్ and ం records gain a cited order; their Unicode
  carrier-first composition order is unchanged. `bengali.json`'s ং gains a
  cited order. Tamil gains its thirtieth letter row, ஸ
  (`tamil.d/letters/0270-U-BB8.json`), with its owner declaration and
  generated owner evidence. ై stays uncited and out of the marks: its
  recognizer class holds only the length mark below.
- **Rewritten:** the thirteen lessons' Writing blocks swap the "copy what you
  see" disclaimer for the numbered-strip wording each track already uses.
- **Pins.** `tests/filmstrip-target-counts`: telugu 45 -> 56, bengali 9 -> 10,
  tamil 65 -> 66. `the-real-corpus` lists the eleven Telugu bare-sign targets
  (and TE-S136 ై, TE-S131 ృ, TE-S133 ౌ as undrawn), Bengali ং, and Tamil's
  one-glyph lessons at 39 for 37 glyphs. New case
  `telugu-signs-drawn-alone`. Evidence: `telugu.evidence.ts` pins the eleven
  cited marks (class, lifts, counts-only wording, the two split loops);
  `bengali.evidence.ts` cites ten rows; new `tamil/letters/0270-U-BB8`;
  `tamil-inventory-ownership` and `script-owner-declarations` count 30 Tamil
  letters.
- **Regenerated:** figures and their hash manifests, the rewritten lessons'
  chapters, narration and modality manifests, and the book hashes.
