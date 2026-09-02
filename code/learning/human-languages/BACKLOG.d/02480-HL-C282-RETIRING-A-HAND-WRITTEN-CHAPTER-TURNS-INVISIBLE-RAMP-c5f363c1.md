## HL-C282 — retiring a hand-written chapter turns invisible ramp debt into visible ramp debt, and that is the point

Italian Chapter 1 was hand-written LaTeX. Flipping it to generated moved the
corpus counter `chapters above 12 atoms` from 27 to 28, and `atom-measurement
blind` from 409 lessons to 401. Both numbers moved because of the same eight
lessons, and reading only the first one would get the change reverted.

The eight were schema-v1. A v1 lesson declares no knowledge atoms at all, so it
contributes **zero** to every atom budget in the corpus — not "a small amount",
zero. Chapter 1 has always taught twenty atoms: five greetings, an adjective
that agrees, two article genders, a noun, three sound points and the etymologies
that carry them. The chapter budget is twelve. It was over budget the whole
time, and the report said nothing, because a lesson that declares nothing cannot
be over anything.

So the honest reading of `27 -> 28` is not *this change made Italian steeper*.
It is *this change made Italian measurable, and it turns out it was already
steep*. The same trap sits under `migrate_schema_v2.py`, which assigns exactly
one atom per lesson and says so in its own docstring: ten lessons, ten atoms,
under the twelve budget, nothing flagged, nothing true. Chapter 1 was migrated
with hand-authored atoms instead — one per teaching section — precisely so the
number that appears is the number the reader actually meets.

**What is now owed.** Chapter 1 should be two chapters: the greeting and its
writing runway, then the day-parts and the courtesy word. That is the ramp
policy's own answer — split rather than compress — and it was kept out of this
change for the reason the Malayalam entry above records: a chapter split
renumbers everything downstream of it, and Italian lesson ids carry the chapter
number (`IT-C22-caffe`). It wants a PR of its own.

**A smaller one, parked with it.** `IT-C01-sera` and `IT-C01-notte` each teach
two headwords — the noun and the compound greeting built from it
(*sera*/*buonasera*, *notte*/*buonanotte*) — while *giorno* and *buongiorno*
get a lesson each. Splitting the two would make Chapter 1 one-new-headword-per-
lesson throughout. It adds two lessons, and adding lessons is what ripples into
`language-ladder`, so it belongs with the chapter split rather than ahead of it.

**One thing to check before the next flip.** The parity script counts prose
BLOCKS and classifies `tabular` as layout, not prose. Italian's hand-written
`notte` section carried a five-language sound-correspondence table that the
lesson had already, deliberately, replaced with a one-change-only sentence.
Parity reported a gap of zero and was right to; the table still disappeared. A
green parity check answers "is a block about to vanish", never "is the chapter
about to get worse".
