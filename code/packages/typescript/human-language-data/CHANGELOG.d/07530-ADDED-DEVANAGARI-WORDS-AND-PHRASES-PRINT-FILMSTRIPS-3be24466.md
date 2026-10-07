### Added — Devanagari ā words and phrases print filmstrips

- `figure-targets.ts`: a shared-headline word may hold the ā sign straight
  after a consonant (`HEADLINE_WORD_SIGNS`, held to script-ductus's table by
  a test). New `headlinePhraseOf`: a headword of two or more such words (or
  one-letter words) separated by single spaces and nothing else is one
  candidate whose `letters` are its words, with `composition:
  "shared-headline"`. Labels, sentences and lists with punctuation stay
  refused. A phrase is capped at `MAX_SEQUENCE_PIECES` letters and signs and
  at the new `MAX_PHRASE_WORDS` (3): measured with the cited letters that
  draw the most movements in the narrowest words, three words print 1,571
  units tall and four 2,048, against 1,801 for the tallest printed strip.
- `figure-filmstrip.ts`: a sequence strip can be a strip of WORDS
  (`SequenceUnit` "Word", passed in by the caller, never guessed): "How it
  is written — 2 words, one after another", "Word 1 of 2 — …", and a
  `<desc>` that says each word is drawn as its letters' bodies, then one
  headline over that word. Only a word strip adds the unit to its source
  hash, so no existing figure changes.
- `figure.ts` / `figure-cli.ts`: a shared-headline target with `letters`
  renders as a word strip; its words must spell the glyph back, one space
  apart.
- Ten lessons gain a strip: HI-S06-vowel-sign-aa, HI-A1F01-name-label,
  HI-W12-schwa-drop, MR-W01-aa-matra, MW-W01-aa-matra, MW-W01-saa,
  SA-S06-vowel-sign-aa and SA-W03-mama-nama-guided-copy, -delayed-copy and
  -dictation. `filmstrip-target-counts`: hindi 54 -> 57, marathi 49 -> 50,
  marwadi 39 -> 41, sanskrit 51 -> 55.
- `data/scripts/devanagari.json`: the ā mark record gains its stroke order
  (class 47), pen lifts, components and its cited place
  (`compositionOrder`, `compositionSource`). `devanagari-marks.evidence.ts`
  pins it.
- Tests: the real-corpus case pins the four ā sign lessons, the three
  words, the three phrases and the refused neighbours (हो, नमः, labels,
  sentences, मेरा); the Devanagari word case covers ā's place, every
  phrase refusal and the caps; the sign case allows a place on ā and the
  nukta only; the sequence-strip suite covers the word unit.
