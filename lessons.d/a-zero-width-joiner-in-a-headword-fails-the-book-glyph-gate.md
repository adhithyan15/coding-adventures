---
category: Testing & coverage
---

# A zero-width joiner in a headword fails the book glyph gate because the generator leaves it outside the script font

The Kannada A2 word list arrived with six English loanwords spelled with a
zero-width non-joiner (U+200C) at the halant: ಡೌನ್‌ಲೋಡ್, ಅಪ್‌ಲೋಡ್, ವೆಬ್‌ಸೈಟ್,
ಪಾಸ್‌ವರ್ಡ್, ಫುಟ್‌ಬಾಲ್, ಸೂಟ್‌ಕೇಸ್. It is legitimate orthography, it passed the
targeted tests, and the strict book compile was clean. But the book generator
wraps each run of script characters in the script's font command, and U+200C
is not a script character, so it splits the run: `\kn{ಡೌನ್}‌\kn{ಲೋಡ್}`. The
joiner lands in the main font, and `tests/glyph-coverage.test.ts` ("THE
GATE: every character in every generated book renders") names every such
chapter. Only the full suite catches it.

**Fix:** write those words the ordinary way, without the joiner (ಡೌನ್ಲೋಡ್ and
so on), and keep U+200C/U+200D out of the candidate filter's allowed set, so
a drafting agent's ZWNJ is rejected before it reaches a lesson. Persian
already avoids the ZWNJ by rule; Kannada, Malayalam and Telugu need the same
rule. A word-list drafted for any script track should be scanned for Cf
characters before generation, not after the full suite.
