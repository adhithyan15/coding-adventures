### Added — HL11 Letter Ledgers: what order to meet the letters
- `data/scripts/<script>-ledger.json` records, for Tamil, Telugu, Kannada,
  Malayalam and Devanagari, the order a reader meets the letters — ordered by
  the words each one makes writable rather than by the traditional recitation
  order, which front-loads independent vowels that unlock almost nothing.
- Measured over the six Indic tracks' opening lessons: this reaches **Tamil's
  "thank you" at the tenth glyph and its greeting at the eleventh**, and
  Devanagari's नमस्ते at the twelfth. The same walk in recitation order completes
  **zero** words after twelve glyphs. Roughly a third of each track's opening
  vocabulary is writable within 24 positions.
- `propose_letter_ledger.py` computes a candidate order and shows its work.
  Families are extracted mechanically wherever a script file's `components`
  already name another letter ("ध: like द with an extra inner loop"); a family
  stated only in prose carries the sentence that justifies it. Not one
  target-script character is typed into the generator — every glyph is looked up
  by its official Unicode name, so a maintainer who cannot read the script can
  audit each line.
- `validateLetterLedger` checks a committed ledger against the corpus and never
  rewrites it. The check that earns its keep is that every claimed unlock names
  a lesson that exists: a ledger and a curriculum drift apart silently, and the
  ledger keeps asserting a payoff long after the lesson delivering it was
  renamed.
- Two rules the payoff ordering may not override: a vowel sign cannot precede
  the first base letter, because in an abugida a mark modifies a letter and a
  ledger opening on one describes a lesson that cannot be written down; and
  letters that share a shape are taught together.
- `chapter-policy.json` gains the drizzle itself — one new letter per script
  segment, at least two lessons between segments, and the unspent-letter window.

