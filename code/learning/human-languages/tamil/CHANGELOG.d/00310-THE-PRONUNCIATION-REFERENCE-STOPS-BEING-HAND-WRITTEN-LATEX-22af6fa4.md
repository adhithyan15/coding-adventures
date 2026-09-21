## The pronunciation reference stops being hand-written LaTeX

`tamil/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `tamil/pronunciation-reference.md`. The
chapter title, the contents line and the running head keep their three separate
strings, so the head over the page still reads "Tamil script" rather than
"Pronunciation".

**Restored from the LaTeX before the flip:**

- The claim the section was actually about. The LaTeX heading read **"No
  Sanskrit-only letters in the core set."**; the Markdown had retitled it "A
  small native alphabet," which describes the size of the alphabet and drops the
  point about Sanskrit. The heading is back.
- The composition step **க ka + ி → கி ki**. The Markdown gave கி as a finished
  result and never showed a vowel sign being hooked on, which is the one thing
  fact 2 exists to demonstrate.

**Kept from the Markdown:** the ௌ (au) vowel sign, which the LaTeX's sign list
was missing even though its independent-vowel list had ஔ au — the two lists
contradicted each other, and the Markdown had the complete one. Also the
gemination example க் + க → க்க, the glosses on the four grantha letters, and
the naming of Dravidian as a family separate from Indo-European, which this
track's LaTeX never said although its three sisters' did.

**Reshaped:** the ten vowel signs were a bullet list nested inside numbered fact
2. A reference's numbered list reads an indented line as a continuation of the
item, so that nested bullet printed as a literal hyphen mid-sentence. The signs
are one middot-separated run now.

