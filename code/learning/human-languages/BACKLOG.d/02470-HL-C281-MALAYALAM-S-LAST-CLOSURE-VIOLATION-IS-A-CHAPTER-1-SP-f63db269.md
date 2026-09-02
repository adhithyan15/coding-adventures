## HL-C281 — Malayalam's last closure violation is a Chapter 1 split, and the arithmetic says so

The Malayalam drizzle re-sequencing took script closure from **19 violating
lessons to 1**. The survivor is `ML-C01-practice`, and it cannot be cleared by
adding letters, because the constraint is arithmetic rather than authorial.

`ML-C01-practice` is the Chapter 1 recap. It prints all five of the chapter's
words back to the reader, so its body is load-bearing for **ഇ**, **ല** and
**ശ** — the letters of **ഇല്ല** and **ശരി**, which the chapter's own last two
word lessons introduce. Closure is measured in reading order, so those three
letters have to be taught inside Chapter 1, after `ML-C01-sari` and before
`ML-C01-practice`.

Chapter 1 has no room. It carries **12 of its 12** permitted new atoms: nine
from the eight-lesson `ML-W01` greeting runway, two lexical, and one for the
newly inserted **അ**. A letter lesson costs one atom, three letters cost three,
and the chapter would land at 15 — which trades a zero in script closure for a
new one in `atomChapterSpikes`, where Malayalam is currently clean. Teaching
all three in a single segment is worse: it breaks the one-letter-per-segment
rule and forward-references both words.

The honest fix is the one the ramp policy names: **split rather than
compress**. Chapter 1 becomes two chapters — the greeting and its writing
runway, then the four short response words with their own letters and recap —
and Chapters 2–66 renumber by one.

That renumber was deliberately kept out of this change because it is not free.
Malayalam lesson ids and concept-atom ids both carry the chapter number
(`ML-C32-tinnuka`, `ML-CONCEPT-C32-TINNUKA-01`), and
`tests/chapter-references.test.ts` pins Malayalam at **46 cross-chapter prose
references** whose numbers would silently rot. The test's own note says what to
do: *"When a track starts splitting chapters, clear it first and move it to
zero."* So the split is a three-part job — clear the 46 prose references to
named landmarks, renumber, then split — and it wants a PR of its own.

Two smaller items were parked for the same reason. `ML-C06-dative-subject`
(325s before trimming) and `ML-C26-raavile` (314s) are both already at the
five-minute ceiling, so neither could take the *Script check* review block the
other five opening chapters received; **ക**, **ര** and **ഭ** are therefore
still reviewed only inside their own lessons, and Chapter 6's payoff
representativeness stays at 0.43. Both are lesson splits, not compressions.
