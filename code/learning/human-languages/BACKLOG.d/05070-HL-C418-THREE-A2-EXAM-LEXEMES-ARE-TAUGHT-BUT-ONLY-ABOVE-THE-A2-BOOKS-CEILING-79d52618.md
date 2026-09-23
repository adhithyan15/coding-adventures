## HL-C418-79d52618 — Three A2 exam lexemes are taught, but only above the A2 book's ceiling

**Status: CLOSED (2026-09-23).** Found while picking the wordlist for the second Spanish A2
vocabulary tranche, by checking every candidate against the corpus before
drafting rather than after.

`explicar` is the highest-value word left in the A2 gap — three failing items
depend on it, and it is the only untaught lexeme still appearing in more than
one item once chapters 437-439 land. It is also **already a
headword**, in `ES-C41-explicar`. So is `creer` (`ES-C41-creer`), and so is
`problema` (`ES-C268-problema`).

The audit is not wrong. It is **book-bounded**: its taught set comes from
`lessonsUpToLevel("A2")`, and all three lessons derive to **B1**, because their
path segments name `SPINE-GIVE-REASONS`, whose stage is `B1`. A reader who has
finished the A2 book has genuinely not met them.

| lexeme (all derive to B1) | failing items |
|---|---|
| explicar — `ES-C41-explicar` | 3 |
| creer — `ES-C41-creer` | 1 |
| problema — `ES-C268-problema` | 1 |

**Do not teach them again.** A second lesson for a word the corpus already has
is duplication the reader would meet twice, and it would make the gap number
fall for a reason that is not real progress.

`ES-PATH-037` holds exactly four lessons — `creer`, `así que`, `deber`,
`explicar` — and the segment is reasonably B1 *as a reason-giving segment*:
*deber*, *así que* and *creer que* are discourse moves an A2 candidate is not
asked for. The mismatch is that a plain vocabulary item, `explicar`, rides along
with them and inherits a level chosen for the grammar around it.

Options:

1. **Split the segment.** Move `explicar` (and probably `creer`) to an A2
   segment, leaving the reason-giving grammar at B1. Smallest honest fix; it
   changes what the A1 and A2 books contain, so it wants its own review.
2. **Give the A2 tranche a lesson that re-teaches nothing** — a chapter whose
   words are new but which *practises* `ES-LEX-EXPLICAR-07`. This does not help:
   the audit's taught set is built from **headwords**, not from practised atoms,
   so the gap would not move and the reader would still meet the word first at
   B1.
3. **Leave it.** Three lexemes and five items out of 161 and 88. Cheap to
   defer, and the measurement above is what makes deferring a decision rather
   than an oversight.

Worth doing with HL-C417, since both are about a lesson inheriting a level from
a spine node chosen for something else.

---

### CORRECTION (measured after filing, before acting)

**Do not do option 1. The premise above is wrong for the word that mattered.**

This entry claimed the mismatch was that "a plain vocabulary item, `explicar`,
rides along with [the reason-giving grammar] and inherits a level chosen for it",
and recommended splitting `ES-PATH-037`. Checking the lesson before acting on my
own recommendation shows otherwise.

`ES-C41-explicar` declares `prerequisites: [ES-C41-deber, ES-C41-asi-que,
ES-C41-creer]`, requires `ES-LEX-DEBER-05`, `ES-LEX-ASI-QUE-03`,
`ES-LEX-CREER-01` and `ES-LEX-PORQUE-03`, and **introduces
`ES-GRAMMAR-REASON-CHAIN-09`**. It is not a vocabulary item riding along — it is
the **capstone** of the reason-chain unit, and the grammar atom is its own.

Tracing each word to the lowest level it could honestly sit at, by the level of
the lessons introducing its required atoms:

| word | lowest honest level | why |
|---|---|---|
| **explicar** | **B1** | needs `deber`, `así que`, `creer` — all B1 |
| creer | A2 | needs only `pensar`, already A2 |
| problema | A1 | needs only `habitación`, already A1 |

So **`explicar` cannot move.** Putting it in an A2 segment would place a lesson
in the A2 book that depends on three B1 lessons — precisely the failure
`lessonsUpToLevel` exists to prevent, and worse than leaving it where it is.

**And the value was overstated.** Crediting all three would have taken
`objectiveFailed` 40 → 38. Crediting only the two that can actually move takes
it **40 → 39** — one item, for a re-mapping of two lessons across two segments,
one of which would pull a B1-segment lesson into the **A1** book and needs its
own pedagogical justification. That is a bad trade, so it is not being made.

**What is actually true, and what it would take.** `explicar` is the only lexeme
appearing in more than one remaining failing row, so it is still the single
highest-value word in the A2 gap. Reaching it means deciding that the whole
reason-giving unit — *creer*, *así que*, *deber*, *explicar*, and the
`SPINE-GIVE-REASONS` node itself — belongs at A2 rather than B1. That is a
**curriculum judgement about the ladder**, not a mapping tidy-up, and it should
be argued on whether an A2 candidate is expected to give reasons at all, never
on the two exam items it would buy.

**Two smaller inaccuracies in the original body, also corrected here.**

The paragraph above says all three lessons derive to B1 "because their path
segments name `SPINE-GIVE-REASONS`". That is true of `explicar` and `creer`
only. `problema` sits in `ES-PATH-268-PROBLEM`, which names
`SPINE-HANDLE-TRAVEL` — a different node that is *also* B1. The conclusion
survives, the attribution does not, and the sentence generalised from two cases
to three without checking the third.

And the one item that the movable pair buys comes entirely from `problema`.
Crediting `creer` alone clears nothing, because its row is also missing
`descontar`. So the achievable gain is one item from one word.

**Method note.** The error was writing a recommendation from a lesson's *list
entry* rather than from the lesson. The list said "vocabulary item"; the
frontmatter said "capstone with three prerequisites and its own grammar atom".
One `cat` of the file was the difference, and the shard had already named
`ES-PATH-037`'s four members without anyone reading what the fourth one does.

---

### RESOLVED (2026-09-23) — closed as "leave it", option 3

All three lexemes stay where they are, and the reasoning is now measured rather
than argued.

| lexeme | verdict |
|---|---|
| `explicar` | **Cannot move.** The CORRECTION above establishes it: capstone of the reason-chain unit, three B1 prerequisites, its own grammar atom. Reaching it means staging `SPINE-GIVE-REASONS` at A2, which is a curriculum judgement about the ladder. |
| `creer` | **Clears nothing.** Its row also misses `descontar`. |
| `problema` | **Attempted via HL-C420 and reverted.** Worth exactly one item, and the honest fix turned out to require re-owning three concepts on the shared 23-language spine. See that entry's measured post-mortem. |

The A2 gate was re-measured with the `problema` fix applied: `objectiveFailed`
48 → 47, and the A1 gate held at 0. So the entry's arithmetic was right and its
cost estimate was not — and the cost is what decided it.

**Both routes out of this entry are the same question**, which is the reason to
close it rather than keep it open as a task: whether `SPINE-GIVE-REASONS` belongs
at A2, and whether `SPINE-HANDLE-TRAVEL`'s concepts belong to it at all. Neither
should be settled by how many exam items it moves. Re-open under a spine-staging
heading if that argument is ever taken up.
