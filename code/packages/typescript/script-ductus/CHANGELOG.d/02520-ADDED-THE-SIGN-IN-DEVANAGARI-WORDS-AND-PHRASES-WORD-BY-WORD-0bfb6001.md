### Added — the ā sign in Devanagari words, and phrases word by word

- `src/strokes/devanagari.ts`: a cited ductus for the ā sign (ा), the ninth
  Devanagari sign: "draw the stem straight down", then "lift, then draw the
  shirorekha rightward" over the piece of headline Noto Sans Devanagari
  prints on the sign (x 0 to 273). Its strokes cite HP Labs India's LipiTk
  Devanagari recognizer, class 47 (55 of 81 native prototypes are the stem
  alone, 54 of those downward; 23 are the stem and then a left-to-right top
  stroke; the signs were collected without a headline). Drawing the headline
  piece last, as the two-stroke writers and the cited आ do, traces the whole
  printed sign, which the stem-only path did not (9.7% untraced).
- `src/headline-word.ts`: `composeHeadlineWord` now accepts ā straight after
  a consonant (क to ह). Its stem is drawn after the consonant's body, and its
  piece of headline becomes part of the word's one headline. The sign's
  place is cited on its mark record (`compositionSource`: the cited आ, which
  draws the same bar after its body and before its headline; HP Labs' आ
  prototypes 52 of 83 body, bar, headline; KanoAI's Gujarati ા after its
  consonant in 32 of 32), at medium confidence, and read from there by the
  new `HEADLINE_WORD_SIGNS`. The word's citation names the sign by position
  and adds its place ("sign 2: …; the place of sign 2, after its consonant's
  body: …"); a word of bare letters reads as before. Every other sign is
  still refused, and the refusal reason now says "is neither a single base
  letter nor a sign with a cited place in a word".
- `src/headline-word.ts`: `composeHeadlinePhrase(phrase, script, font)`
  composes words separated by single spaces and nothing else, each on its
  own with its own headline (a one-letter word is that letter's own strip),
  and refuses the whole phrase naming the first word that fails.
- `tests/filmstrip-ledger.test.ts`: a shared-headline candidate with
  `letters` is a phrase and contributes one entry per word, only when every
  word composes. The real-corpus pin now holds नाम (HI-A1F01-name-label,
  HI-W12-schwa-drop), सा (MW-W01-saa) and the phrase मम नाम (three Sanskrit
  lessons) beside मम. A new case renders the tallest phrase the book's cap
  allows (औइ औझ धऋ, 1,571.14 units) and holds it under the tallest printed
  strip (1,801.14); a fourth such word would print over it. The Devanagari
  ledger owner gains `devanagari:ा`, `devanagari:नाम` and `devanagari:सा`.
- Pins: `stroke-ownership` keys 635 -> 636, `devanagari: 53`, with new
  key and non-Tamil data hashes; the sign tests add ā (class 47, one lift)
  and drop it from the left-out list.
