## HL-C241 — Sanskrit's remaining 31 closure violations are cadence-bound, not order-bound

The ladder was rebuilt and moved to chapter one: 48 segments, one new Devanagari
character each (verified by walking the corpus in reading order), scheduled
against the demand curve. Closure fell **46 → 31** and never-taught characters
**9 → 5**. What is left is no longer an ordering problem, and re-running the
same optimisation will not move it further — the search plateaus at 30 across
four independent seeds.

**The binding constraint is HL11 §4's `minLessonsBetweenScriptSegments: 2`.**
One segment per two content lessons leaves 28 slots before chapter 14, against
roughly 40 characters of demand from chapters 2–13. Measured: at a cadence of
**one segment per content lesson** the same scheduler reaches **25** violations;
at cadence 2 it reaches 30; at cadence 3, 43. So the remaining debt is a
deliberate policy choice — "the script never becomes the course" — and the
honest way to move it further is to argue that policy, not to re-order the
ladder. Do not spend another tranche on the ordering.

**Three violations are a hard floor until `devanagari.json` grows.** `ऋ` alone
blocks `SA-C05-aham-samskritam-vadami`, `SA-C09-grhnati`, `SA-C09-prcchati`,
`SA-C13-hridayam` and `SA-C26-king`, and neither `ऋ` nor `ङ` has an entry in
`data/scripts/devanagari.json` — no components, no cited stroke order. HL11 §5
binds. That file is shared with Hindi, Marathi and Marwadi, so extending it must
come from a branch that owns it, not from a Sanskrit tranche.

**Correction to HL-C217, which is wrong on the facts.** It claims `इ ई घ ड ँ ू ◌ै`
all lack a Sanskrit headword. Measured against the corpus: `इ` is in इदानीम्,
`ड` in क्रीडति, `ू` in सूनुः and `ै` in वैद्यः — all four now have segments and
are taught. The characters that genuinely have **no headword anywhere in the
track** are `ौ ई ँ घ ऋ`. `ौ` keeps a segment (it appears in running text and is
built from two shapes the reader has) and says so plainly; `ई ँ घ` need
vocabulary scheduled before a segment can honestly anchor them, and `ऋ` needs
the shared data file. That is the accurate work queue.

**Two report-only numbers moved the wrong way and were not hidden.** Chapter 7
crosses the 12-atom chapter budget (12 → 15) because three segments land there
and it was already at budget; chapter 6 was over before this tranche (15) and is
now 16 — its three lessons carry 6, 5 and 4 atoms against a per-lesson budget of
3, which is the real debt underneath. And five more chapter payoffs fall below
the representativeness floor (80 → 85) because chapters 1–5 now carry typed
atoms while their legacy schema-v1 payoffs assess nothing. Both are consequences
of putting a typed ladder into untyped chapters; both are fixed by finishing the
schema-v2 migration of chapters 1–5, not by moving the ladder back.

**Script atoms are still under-reviewed, but less so.** Each segment now
declares the previous three segments in `practises.knowledge` and reviews them
by name in its warm-up, taking a `SCRIPT-RECOG` atom from roughly 2 appearances
to 4. `LEX` atoms average ~14. The gap is smaller and still real.

And the three-back window is not free: Sanskrit's `reinforcementWindowMisses`
moves **744 -> 761**. The near windows improve (R1 43 -> 32, R2 238 -> 225) and
the far ones worsen (R3 260 -> 276, R4 203 -> 228), which is exactly what a
three-back review does — it puts every atom back in front of the reader soon and
then never again. Atoms never revisited at all fall 13 -> 12. The fix is a
spaced schedule rather than a fixed three-back one: a segment should also review
an atom from roughly ten and roughly thirty segments back. That is a change to
the ladder's frontmatter generator, not to its ordering, and it is the cheapest
remaining win in this file.

**The driving edition paid for this and the bill is not hidden.** Sanskrit's
chapter-prefix reach fell 240 → 179 lessons and its drivable share 91% → 83%.
That is the cost HL08's placement measurement predicted when it chose
end-of-chapter for the first segments, and it is the direct price of moving the
ladder to where closure can be affected at all. No chapter became unstartable —
every segment has at least two content lessons ahead of it. If the driving
edition is judged more valuable than the remaining 15 retired violations, the
lever is the schedule in the tranche's authoring script, not a re-argument of
the closure rule.
