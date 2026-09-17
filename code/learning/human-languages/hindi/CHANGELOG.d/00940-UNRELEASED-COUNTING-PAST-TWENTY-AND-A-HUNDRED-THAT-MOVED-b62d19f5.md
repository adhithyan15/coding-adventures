## Unreleased — counting past twenty, and a hundred that moved

**Chapter 104 teaches the tens. `HI-A1-NUM-03` is deliberately not probed**, and
that decision is the substance of this entry.

| metric | before → after |
|---|---|
| exam-point coverage | unchanged (237/282, 84%) |
| atoms taught | 534 → 542 |
| measurable lessons | 478 → 484 |
| `forward-language` | unchanged (22) |
| `payoff-surprise` | unchanged (13) |
| `scriptClosureViolations` | unchanged (20) |
| `atomsNeverRevisited` | unchanged (33) |
| `durationViolations` | unchanged (0) |

### What the chapter teaches, exactly

Seven rungs — **तीस, चालीस, पचास, साठ, सत्तर, अस्सी, नब्बे** — and **सौ** in the
money chapter. That covers every price, age and house number that **lands on** a
rung, including *पचास रुपये*, the mock reading item the old note named as
unreadable.

**It is not the whole range, so the point stays open.** Hindi does not build
21–99 from parts a learner can reassemble: **पच्चीस** is not **पाँच** plus
**बीस** in any workable sense, and the seventy-two numbers between the rungs are
each individually eroded. The chapter says so rather than inventing a rule, and
teaches the one thing that does hold — **Hindi puts the small number first**, so
पच्चीस opens with the sound of पाँच, the opposite of English.

Probing the point on the rungs alone would claim a range the corpus does not
teach. `HL-C386` is the precedent for why that is not done.

### The hundred moved, and the move is the point

**सौ was drafted into chapter 104 and belongs in chapter 79.**

`HI-C75-fifth` teaches **सौवाँ** — *hundredth* — and teaching **सौ** at chapter
104 made that ordinal a forward reference. `forward-language` went 22 → 23.

The first draft made a virtue of the oddity: its warm-up opened *"say सौवाँ, now
take the ending off"*, treating "you could name the hundredth thing before you
could count to a hundred" as a hook. **That was dressing up a defect.** Moving
the lesson to the money chapter — beside **रुपया** and **ख़रीदना**, where a price
needs it — puts the number before the ordinal built on it, and `HI-C75-fifth`
now pays it off. The metric went back to 22 because the ordering was fixed, not
because the wording was.

### Two more regressions the snapshot diff caught

- **`scriptClosureViolations` 20 → 21.** The review lesson illustrated *unit
  first* with **इक्कीस**, which opens with the independent vowel **इ** — one of
  the two glyphs the corpus still never draws. Swapped to **पच्चीस**, which makes
  the same point and opens with a letter the reader can read.
- **`payoff-surprise` 13 → 14.** The chapter payoff assessed three of the
  chapter's atoms, below the half-the-chapter floor. The synthesis now reads five
  prices rather than three, and the payoff assesses four.

All twelve gates passed on every one of those. **None of them was visible
anywhere but the snapshot diff.**

### Also

The first draft referenced `HI-LEX-C75-ORDINAL-03`, an atom id that does not
exist — the ordinals lesson introduces `-06`. Caught by reading the target
lesson rather than assuming the numbering.
