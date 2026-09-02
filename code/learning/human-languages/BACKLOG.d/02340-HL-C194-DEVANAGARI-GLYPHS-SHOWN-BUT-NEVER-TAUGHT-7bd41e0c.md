## HL-C194 — Hindi still shows 12 Devanagari glyphs it never teaches

The romanization pass on this branch took Hindi's
load-bearing headwords from 71 to 1 and its script-closure violations from 69
to 40. What it could **not** touch is the residue those 40 violations are made
of, because the exposure rule exempts a lesson's *headword*, never its *body*.

Measured on the current corpus: Hindi shows **57** distinct Devanagari glyphs
and teaches **45**, leaving **12 shown but never taught**. Those twelve are
what keeps 40 lessons in violation — a learner reaches them inside body prose,
tables, and etymology notes, where no romanization exemption applies and no
earlier lesson has introduced the letter.

This is the *right* remaining shape of the debt, and it is not a metadata
repair. Closing it needs real teaching, in the track's established
gloss-first-then-glyph order: identify the twelve, find the earliest lesson
that shows each one, and add a writing step **before** that lesson rather than
retro-fitting a note into it. The W01–W05 writing companions are the existing
home for that work, and the track's own README already names six letters
(**ख ज ञ ट ण ष**) as appearing in prose without entries in
`data/scripts/devanagari.json` — a likely overlap with the twelve, and the
place to start.

Two cautions for whoever picks this up. First, `devanagari.json` is **shared
with Marathi, Sanskrit and Marwadi**; a glyph added there ripples into three
other tracks and must not be edited from a Hindi-only branch. Second, the
README records two known data defects not yet settled — the **ध** entry omits
the vertical spine it actually has, and the **ह** entry asserts a right spine
although ह is traditionally counted in the no-pāī class. Both are load-bearing
for conjunct behaviour, so settle them against a typography source before
teaching from that data rather than after.

Do not close this by declaring headword romanizations. That lever is spent,
and pulling it further would only launder body glyphs through headwords — the
exact laundering the exposure metric's own comment warns about.
