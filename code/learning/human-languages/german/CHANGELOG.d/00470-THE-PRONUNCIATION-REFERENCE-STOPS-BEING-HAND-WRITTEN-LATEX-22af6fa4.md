## The pronunciation reference stops being hand-written LaTeX

`german/book/chapters/appendix-pronunciation.tex` was hand-authored and printed
as a `\chapter*`. It is now rendered from `german/pronunciation-reference.md`.

Every claim survives, and the Markdown adds *ö*, the uvular *r*, the *h* that
lengthens a vowel after it, and *ten* → *zehn* to the consonant shift. Two
things had to be fixed in the Markdown first, and both were only visible on a
compiled page:

- **A stray CJK character.** The vowel-length line read "Doubled/长 vowels are
  long". `长` is Han; the German book loads no CJK font, so rendering this file
  as-is would have put a missing glyph in the middle of a sentence. The line now
  reads "Doubled vowels are long".
- **The consonant-shift table.** A Markdown table inside a reference is rendered
  as one labelled record per row, which is right for the wide Cyrillic and Urdu
  tables it was built for and wrong for three narrow columns: it restated
  "English:" and "→ German:" on all four rows and ran to twelve lines where the
  hand-set `tabular` took five. The mapping is now four bullets — "English **d**
  → German **t**: goo**d** → gu**t**, **d**ay → **T**ag" — which carries the
  column headings inline and reads as the table did.

