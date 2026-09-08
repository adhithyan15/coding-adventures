## HL-C369 — Twenty-two tracks cannot show a learner a paragraph

Measured across all 23 tracks: **three lessons in 6,574 carry `type: reading`**,
and Spanish — the deepest track, 1,155 lessons and 417 chapters — had none. Two of
the three are Japanese body-map lessons and one is a Punjabi assembly drill; not
one of them contains a connected passage.

The cause was structural rather than an authoring oversight. `comprehension` has
been a member of `LessonBlockType` since schema v2 with **nothing on either side
of it**: no authored heading classified to it, and no renderer consumed it. A
passage could only reach a book by being labelled `You'll want to know` and
declared an input block, which is why the three nominal reading lessons contain
no passage. That is the same trap the classifier's own comments call out for
`the letters in this word` and `across the family`.

This entry records the two halves that remain after the Spanish seed lands.

**The measurement is blind to it.** `exam-inventory-es-a1.json` holds 273 points
and **zero** reading points — grep for `lectura`, `comprensión`, `texto` returns
nothing. DELE A1 has a *Comprensión de lectura* paper. So a track can report
100% A1 exam coverage while a learner sitting the paper has never read a
paragraph, and the corpus-wide figure of 2,977/5,151 cannot see the gap. The
inventories need reading points before coverage means what it says.

**Twenty-two tracks still have no passage.** The renderer now supports one in
any track at no cost, since the box is inline and needs no `preamble.tex` edit.
The blocker is per-track: a passage may only use words that track has already
taught, so each one has to be built against its own vocabulary. `luego`-style
connectives and a stock of concrete nouns are the practical prerequisite, which
several tracks do not yet have.

Note for whoever picks this up: a reading skill introduced once **revokes the
track's level**. `ES-SKILL-CONNECTED-READING` introduced in a single lesson
dropped Spanish from A1 to pre-A1 on the reinforcement criterion, because an
atom revisited fewer than twice is not taught. Reading arrives in threes or it
does not arrive.
