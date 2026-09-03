## HL-C317 — the narration stage directions are stripped in one block kind and printed in every other

`GE-C08-mittag-mitternacht`'s warm-up carried two `[YOU SAY: …]` lines, put there
to make the block assess two atoms it was already practising. They reached the
printed book verbatim:

    {[}YOU SAY: the borrowed one, one long syllable --- \textquotedblleft{}Uhr\textquotedblright{}{]}

The generator strips `[YOU SAY: …]`, `[PAUSE Ns]`, `[REPEAT xN]` and
`[YOU HEAR: …]` when they appear as items of a **Guided Practice bullet list**,
and nowhere else. A stage direction in prose, in a warm-up, in a recall block, or
in a bullet list under any other heading is escaped and typeset as literal text
with visible braces.

That is not a German problem. Measured across the corpus at the time of writing:

| marker | occurrences printed |
|---|---|
| `[YOU SAY: …]` | 69 |
| `[PAUSE Ns]` | 56 |
| `[YOU HEAR: …]` | 37 |
| `[REPEAT xN]` | 30 |
| **total** | **192** across **61 chapter files** in **12 tracks** |

Spanish (14 files), Marathi (9), Chinese (8) and Japanese (8) carry the most.
German carries none, because this entry is being filed alongside the fix to the
one German instance.

**Why it is worth an entry rather than a sweep.** The obvious repair — teach the
book generator to strip the markers everywhere — is one small change and would
silently rewrite 61 generated chapters across twelve tracks that four agents are
authoring concurrently. The other repair — rewrite the 192 source lines as prose
— is the right answer per lesson but is 192 authoring decisions in someone else's
material.

**The generalisable part.** A marker vocabulary that is only honoured inside one
block kind is not a marker vocabulary; it is a formatting convention for one
list. Either the stripper runs over every block, or the lesson schema has to
reject the markers outside Guided Practice so an author finds out at validation
time instead of in the PDF. Right now nothing fails: `check:books` regenerates
happily, XeLaTeX compiles without a warning, and the defect is visible only to
someone reading the rendered page.

**How it was found.** Not by a gate. A security review of the chapter-12 diff read
the generated `.tex` beside the lesson source and noticed the two did not say the
same thing. Every automated instrument in the pipeline agreed the chapter was
fine.
