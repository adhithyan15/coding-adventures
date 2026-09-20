## The pronunciation reference stops being hand-written LaTeX

`telugu/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `telugu/pronunciation-reference.md`. The
chapter title, the contents line and the running head keep their three separate
strings, so the head over the page still reads "Telugu script" rather than
"Pronunciation".

**Restored from the LaTeX before the flip:**

- **Eight worked syllables.** The LaTeX spelled out కా కి కీ కు కూ కె కే కొ కో;
  the Markdown had reduced all but కా to bare signs. Both are on the page now —
  the syllables first, because that is what a reader looks up, then the signs
  themselves.
- **The reason Telugu keeps the Sanskrit aspirates**: *because it borrowed so
  many Sanskrit words*. The Markdown stated the fact and dropped the cause.

**Corrected:** the conjunct example was arithmetic that did not add up. Both the
LaTeX and the Markdown wrote **స + క → స్క**, with the virama appearing in the
answer from nowhere — in a numbered fact whose whole subject is that the virama
strips the vowel. It now reads **స్ + క → స్క**, which is how the Hindi reference
has always written the same rule. The virama glyph ్ is also shown, having been
named twice and printed never.

**Reshaped:** the nested vowel-sign bullet, as above.

