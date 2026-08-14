# HL13 — The spine, laid out in Spanish; the script addendum, laid out in Tamil

**Status:** specification, 2026-08-14
**Method directive (owner, 2026-08-14):** *"I want to layout the spine with
Spanish and then layout the script work addendum in Tamil and then replicate it
across all the languages."*
**Goal directive (owner, same day):** *"For every language I want to start with
pre-A1 level (absolute beginner) to C2."*

---

## 1. Why two reference tracks rather than six in lockstep

The previous method advanced six Indic tracks together on every tranche. It has a
defect that showed up three times in two days: **every design mistake is made six
times and fixed six times.** Lexicon wave I put its chapter at the end of the book
in all six; its forward references appeared in all six; a duplicated block type
appeared in all six. Each was one decision, replicated before it was tested.

One reference track per concern inverts that. A pattern is authored once, proved
end to end — through the gates, into the book, read back from the PDF — and only
then replicated. Replication carries a pattern that has already survived contact.

| | reference | why that track | replicated to |
|---|---|---|---|
| **SPINE** — the meaning ladder | **Spanish** | furthest up the ladder, and the only track with an exam-coverage gate that can say a rung is *passed* rather than *touched* | all 22 tracks |
| **SCRIPT ADDENDUM** — the drizzle | **Tamil** | the only track with a **cited** stroke order, so the addendum can be built whole rather than stubbed | every non-Latin track |

This changes the method, not the goal. Every language still climbs pre-A1 → C2.

---

## 2. Where the spine actually stands

`core/spine.json` holds **33 nodes**. Measured against Spanish's own path:

| stage | nodes | Spanish realizes |
|---|---:|---:|
| pre-A1 | 7 | **7** |
| A1 | 4 | **4** |
| A2 | 5 | 4 |
| B1 | 5 | 3 |
| B2 | 4 | 2 |
| C1 | 4 | **0** |
| C2 | 4 | **0** |
| | **33** | **20** |

Two facts follow, and they point in opposite directions.

**The bottom of the ladder is built.** pre-A1 and A1 are fully realized, and
Spanish has 412 lessons across 266 chapters standing on them. Whatever the
reference layout is, the lower rungs already demonstrate it.

**The top of the ladder does not exist in any language.** C1 and C2 have eight
nodes between them and **zero** realizations in the corpus's most advanced track.
The eight are not vague, either — they are the specific things that separate an
advanced reader from a fluent one:

```
C1  infer implicit meaning · structure extended text · shift register
    · follow regional variation
C2  summarize from sources · express fine shades · read literary and older texts
    · read the cultural weight of a phrase
```

## 2.1 Thirty-three nodes is a skeleton, not a ladder

HL09 §3 puts a complete track near 8,000 lessons. Thirty-three spine nodes cannot
carry that; they are the *load-bearing frame*, and each one needs many rungs
hanging off it. Spanish's 412 lessons hang off twenty nodes — roughly twenty
lessons per node at the bottom of the ladder, and that is the shape the top needs
too.

So laying out the spine means two different jobs, and confusing them is how this
stalls:

1. **Realize the thirteen unrealized nodes in Spanish.** Each needs its first
   rung — the lesson sequence that makes its `canDo` true.
2. **Deepen each node to the density the lower ones already have.** This is where
   the page count goes, and where the owner's *"do not worry about the number of
   pages"* is doing real work.

---

## 3. What a rung must contain, as Spanish will demonstrate it

Every realized node in the reference track shows the same anatomy, so replication
has something definite to copy:

| part | what it is | already gated by |
|---|---|---|
| **entry** | the lessons that first make the `canDo` true | curriculum path node |
| **vocabulary floor** | the cumulative word count that rung assumes | HL09 §3 targets |
| **grammar points** | the inventory items the rung teaches | HL-C128's Plan Curricular gate |
| **text shape** | the length and kind of connected text the reader handles | HL-C129 reading closure |
| **task shapes** | what the reader is asked to DO with it | HL-C130 |
| **payoff** | the chapter capability that proves it | HL05 |

A rung is finished when all six are present and the gates read clean for it —
not when lessons exist under its node.

---

## 4. The replication contract

A pattern proved in a reference track replicates as a **generator plus a per-track
review**, never as six hand-authorings. Three properties travel with it and are
not renegotiated per track:

- **The ramp stays gentle.** Every HL08/HL11/HL12 budget applies to the replica.
- **The script drizzles in beside the meaning**, never in front of it. The book
  is useful from page 1 in every track, not only in the reference.
- **Page count is never a constraint.** *"We can always split the book in the
  future."* A rung that needs forty lessons gets forty.

What does NOT replicate automatically is anything requiring a citation. Tamil's
script addendum carries cited stroke orders; a track without them gets the
addendum's *recognition* half and reports the missing pen paths as debt, per
HL11 §5. **No citation → no pen path → no figure**, in every track, forever.

---

## 5. Order of work

1. **Spanish**: realize the thirteen unrealized nodes, lowest stage first — A2's
   one, then B1's two, then B2's two, then C1's four, then C2's four.
2. **Spanish**: deepen each rung to the density the pre-A1 and A1 rungs have.
3. **Tamil**: complete the script addendum end to end — the letter ledger's 24
   positions, conjuncts, running text, and the lesson where decoding closes.
4. **Replicate the spine layout** to the other 21 tracks, one stage at a time.
5. **Replicate the script addendum** to every non-Latin track, recognition-first
   where the ductus is uncited.

Steps 1 and 3 are independent and alternate, per the owner's direction of
2026-08-14.

---

## 6. Provenance

| claim | source |
|---|---|
| 33 spine nodes, stage distribution | `core/spine.json`, 2026-08-14 |
| Spanish realizes 20 of them; C1 and C2 at zero | walk of `spanish/curriculum.json` against the spine, same date |
| 412 Spanish lessons, 266 chapters | corpus count, same date |
| ~8,000 lessons for a complete track | HL09 §3, carried forward |
| every mistake made six times | lexicon wave I: placement, forward references, duplicated block type |
| the two-reference-track method | owner, 2026-08-14, quoted in the header |
