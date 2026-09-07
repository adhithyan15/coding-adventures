### Changed — the four Dravidian pronunciation references are generated

- tamil, telugu, kannada and malayalam render their `appendix-pronunciation.tex`
  from `<track>/pronunciation-reference.md`, each reusing the comparison script
  set its chapters already use, and each keeping its own running head.
- Twenty-five worked syllables that only the LaTeX had were carried into the
  Markdown, along with three prose claims: Tamil's "no Sanskrit-only letters in
  the core set", Telugu's reason for keeping the aspirates, and Kannada's "one of
  the four literary Dravidian languages".
- **The conjunct examples were wrong in three tracks, in both the LaTeX and the
  Markdown.** `స + క → స్క` writes the virama into the answer without it
  appearing in the inputs — inside the very fact that says the virama strips the
  vowel. Telugu, Kannada and Malayalam are corrected to `స్ + క → స్క`, matching
  how the Hindi reference has always written it, and each now shows the virama
  glyph it previously only named.
- Tamil's LaTeX sign list was missing ௌ (au) while its independent-vowel list had
  ஔ au. The Markdown's complete list wins.
