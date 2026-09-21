## HL-C418-79d52618 — Three A2 exam lexemes are taught, but only above the A2 book's ceiling

**Status: OPEN.** Found while picking the wordlist for the second Spanish A2
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
