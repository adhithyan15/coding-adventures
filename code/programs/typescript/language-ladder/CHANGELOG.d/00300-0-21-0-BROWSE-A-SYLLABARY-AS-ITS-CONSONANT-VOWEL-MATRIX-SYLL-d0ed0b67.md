## 0.21.0 — Browse a syllabary as its consonant × vowel matrix (syllabary, PR 5)

- **The syllabaries now offer a grid view.** A Dravidian abugida isn't a flat
  list of ~450 signs; it's a table — every consonant marching across the same
  vowel row (ka kā ki … , kha khā khi … , ga gā gi …). Browse gains a **List /
  Matrix** toggle (syllabaries only; alphabets stay a plain list): Matrix lays
  the syllables out as **rows = consonants, columns = vowels**, so the abugida's
  regularity is the first thing you see. Clicking any cell selects that syllable
  and opens the existing "break it apart" detail panel. No new data — the same
  generated syllables, re-arranged.
- **New pure helper `buildSyllableMatrix(letters)` in `matrix.ts`.** It reuses
  the grounded consonant boundary from `syllabary.ts` (a new row at each bare
  consonant) and reads the column vowels off the first consonant's own row (its
  base syllable's sound minus its inherent vowel gives the consonant prefix;
  stripping that off each syllable yields the vowel it carries — kā → "ā",
  kr̥ → "r̥"). Nothing is invented. If the rows don't all span the same vowels it
  returns **null** rather than risk a syllable sitting under the wrong vowel
  header. Unit-tested with a **control** that a ragged input yields no matrix,
  plus a check against the real Telugu data (a full 35 × 13 grid; the vocalic-R
  column header is ISO-15919 r̥ = r + U+0325, not the IAST dot-below ṛ).

