---
category: Testing & coverage
---

# A CDIAL row copied into a book carries CDIAL's phonetic letters; screen cited forms for glyphs the main font lacks

**What went wrong.** The Marwadi A2 vocabulary (chapters 166-290) cites a
CDIAL Gujarati row whenever the entry has no Marwari row. CDIAL writes
Gujarati's open vowels with **ɔ** (U+0254) and **ε** (U+03B5): *udhārɔ*,
*ā̃trɔ*, *mā̃ḍvɔ*, *sāḷɔ*, *bhāṇiyɔ*, *vaṇjārɔ*, *celɔ*, *akhāṛɔ*, *vεṇ*. The
drafters copied the rows exactly into nine "taken apart" texts. Every
generation screen passed. The spec screen did check for combining marks, ṁ and
invisible characters, but not for phonetic letters. Then
`tests/glyph-coverage.test.ts` failed on eight book chapters: the main font of
the Marwadi book has neither glyph. This is the same font gap as the Bengali
**nɔ** lesson ("the font a romanization renders in is not the font the script
renders in"), reached by a different route. Here the character came from a
dictionary citation, not from a romanization the author chose.

**Fix.** Write the cited form in the plain romanization the track already uses
for Gujarati's -ો and એ: *udhāro*, *veṇ*, and so on. Make the change in the spec
and in the generated lesson. Then re-run the package generators (books,
narration, modality) so the derived files follow the lesson. No source changed:
the Gujarati word is the same.

**Do differently.** Before generating, screen every printed field of a spec
(examples, hooks, "taken apart" texts, notes) for any non-ASCII character
outside the script wrapper that the book's main font lacks. A CDIAL, Turner or
Monier-Williams quotation carries that dictionary's transcription, which
includes ɔ, ε and other phonetic letters. Treat a pasted citation as untrusted glyph input,
just like a romanization.
