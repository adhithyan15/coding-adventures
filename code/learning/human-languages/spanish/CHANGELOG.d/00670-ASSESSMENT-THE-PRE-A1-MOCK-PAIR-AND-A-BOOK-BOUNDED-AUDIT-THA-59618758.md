## Assessment: the pre-A1 mock pair, and a book-bounded audit that caught five items

`mocks/pre-a1/` now holds what the contract has named since it was written and
never had: a rubric, two timed papers, two answer keys, and a machine-generated
**book-bounded audit**. Spanish's artifact ceiling falls **23 → 20** with no
pre-A1 debt left.

**What the rung is, and is not.** The A1 rubric next door *transcribes* an
awarding body's real rule — DELE A1 v2020, sourced to the Instituto Cervantes
guide, explicitly "does not improve it." There is no DELE below A1, so there is
nothing here to transcribe. This rubric is a **project standard** and says so
rather than borrowing authority it does not have.

**The pass rule is the strict one.** Four papers of 25, and **15 of 25 on every
paper in the same sitting**. 25 in reading and 10 in writing is 60 points and is
*not* a pass. DELE itself groups reading with writing and passes each group at
30/50 — a grouped rule lets a strong reader carry a weak writer, and the whole
claim of this rung is that all four skills moved together.

**The audit is the part that matters.** Every mock paper implies "every word
here is taught" and almost none proves it. `book-bounded-audit.json` parses both
answer keys' `requires` tables against the headword set the book teaches by the
end of pre-A1: **40 objective items, 0 unbounded.**

It caught **nine failures on its first run**, and every one had been cleared by
my own prose search — because that search read lesson *bodies* while the
corpus's definition of taught is the *headword set*. **buenos días**, **la
casa**, **el día**, **estoy** and **¿cómo te llamas?** all appear in Spanish
pre-A1 lessons and **none of them is taught there.** Five items were rewritten
rather than re-annotated.

**What is deliberately absent.** No item on either form turns on the Peninsular
*z*: the rubric grants seseo full credit, so an item a seseante candidate cannot
answer would contradict it. And **no sitting has been run** — the A1 pair
carries one that measured a book-only reader at NO APTO on both forms; nothing
equivalent exists here, and the absence is recorded rather than filled with a
guess.

The audit CLI is now parameterised by level instead of hard-coding A1; the A1
report regenerates byte-identical.

