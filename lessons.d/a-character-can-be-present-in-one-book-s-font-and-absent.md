# A character can be present in one book's font and absent from another's — check the GENERATED .tex against the actual font file (human-language books)

**Context:** HL-C212, the Latin pre-A1 vocabulary tranche. 744 local tests green,
security review clean, pushed. CI failed 11 minutes later with a single line:

    latin missing_character rose to 2 against a baseline of 0

**What happened:** the Old English form of *egg* was written *ǣg* — U+01E3 LATIN
SMALL LETTER AE WITH MACRON — and Latin Modern, which the Latin book sets as its
main font, does not have that glyph. Nor U+01E2, its capital. Exactly two
characters, exactly the two the scanner counted.

**Why no local gate caught it.** Every local check reads the corpus, the schema,
the ramp and the manifests. **None of them opens a font.** Glyph coverage is only
discovered when XeLaTeX runs, which happens in CI, which is 11 minutes away.

**The instrument that would have been wrong.** My first instinct was that the
offender was `ǵ` (U+01F5, in the PIE reconstruction *\*ǵʰórtos*) — it looks far
more exotic. I checked instead of guessing, by loading the actual `.otf` with
`fontTools` and querying its cmap, and **`ǵ` is present**. The guess would have
sent me to rewrite the wrong lesson and left the failure in place.

**And the second wrong instrument, caught the same way.** Scanning the LESSON
SOURCES for characters outside the font reported four more offenders — `ʰ`, `₁`,
`₂`, `ḗ`. All four are false: the book generator turns them into
`\textsuperscript{h}` and `\textsubscript{2}` and decomposes the rest, so they
never reach the `.tex` at all. **The artifact to check is the GENERATED .tex,
because that is what the typesetter reads** — checking the source measures a file
nothing compiles.

**The check, worth running before any content PR that adds an unusual character:**

```python
from fontTools.ttLib import TTFont
font = TTFont(".../lmroman10-regular.otf")
cmap = set()
for t in font["cmap"].tables: cmap |= set(t.cmap.keys())
assert ord("a") in cmap and ord("好") not in cmap   # self-test, both directions
missing = {c for p in book.rglob("*.tex") for c in p.read_text()
           if ord(c) > 127 and ord(c) not in cmap}
```

Self-test it in **both** directions before trusting a clean result — a cmap that
loaded empty reports every character missing, and one that loaded wrong reports
none.

**Generalisation:** per-track fonts mean per-track repertoires. The Indic and
Arabic books load vendored Noto faces with wide coverage; the Latin, Spanish and
other Latin-script books load Latin Modern, which is a **typesetter's** font and
is much narrower outside western European orthography. A character that has
rendered fine in twenty books can still be missing from the twenty-first. Cross-
refs the existing lesson about a character no book has rendered before.

**Cheapest content fix:** use the citation form that stays inside the font.
Old English *æg* (plain ash) is a standard rendering of the same word and is
present in every Latin-script face in the repo.
