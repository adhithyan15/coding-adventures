## The pronunciation reference stops being hand-written LaTeX

`bengali/book/chapters/appendix-pronunciation.tex` was hand-authored and printed as a
`\chapter*`. It is now rendered from `bengali/pronunciation-reference.md`.

**Restored from the LaTeX before the flip:** the macron in *bād* — the Markdown
had written Sanskrit *vāda* → *bad*, losing the long vowel the example is about —
and the claim that Bengali *carries an immense literary tradition*, which the
Markdown had compressed into a bare mention of Tagore.

**Corrected:** the conjunct example read `স্ + ক → স্ক, চ + ছ → চ্ছ`. The second
half lost its hasanta, so the rule the sentence states was contradicted by its
own example. It now reads `চ্ + ছ → চ্ছ`, and the hasanta ্ is shown in the
sentence that names it.

The endonym stays as the Markdown's *Bāṅlā* rather than the LaTeX's *Bānglā*:
বাংলা is written with an anusvara, and *Bāṅlā* is what transliterates it.

