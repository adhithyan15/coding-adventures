## HL-C313 — a paradigm table the info-dump gate cannot see, because its rows are not subject pronouns

French chapter 2 is now generated. **Eight hand-written French chapters remain,
nineteen across the corpus.** The chapter needed no new lessons; it needed one
table removed, and the reason it needed removing is not something any gate said.

### What was there

`FR-C02-me` — the *second lesson of the second chapter* — carried the French
reflexive set as a four-column table:

    | French | means           | root    | English cousin        |
    | me     | myself          | Latin mē| me, my, mine          |
    | te     | yourself        | Latin tē| archaic thee          |
    | se     | himself/herself | Latin sē| the "self" in separate|

The **hand-written chapter never printed it.** Its `grammarlens` wrote the same
three pronouns as one sentence of prose. So the flip would have added a paradigm
to the reader's page rather than preserving one — the opposite of the failure
mode `handwritten_parity.py` exists to catch, and invisible to it for the same
reason: the gate counts blocks that would DISAPPEAR.

### Why no gate caught it

`info-dump.ts` flags a table with three or more **person-labelled** first-column
cells. Its label map is a census of what the corpus's paradigm tables actually
put there, and for French that is the SUBJECT pronouns:

    french: je, j', tu, il, elle, on, nous, vous, ils, elles

`me`, `te` and `se` are reflexive pronouns. They match nothing, `personRowCount`
returns 0, and a genuine three-person paradigm scores clean. The same hole exists
for every track in the map: object pronouns, possessives, and any other
person-indexed series whose forms are not the subject series.

**The gate not flagging a table is not evidence the table is fine.** Read the
tables in every lesson you migrate, and ask whether the rows vary by PERSON —
not whether they begin with a word on a list.

### Two other things the table was doing

- HL10 forbids printing a paradigm table until every cell in it has been taught
  individually. *te* is used once in that chapter and *se* not at all.
- Four columns is over `maxLinearisableTableColumns`, so the narrator **refused**
  it and spoke an apology instead. Removing it took the corpus-wide refusal count
  51 -> 50 — which is the one signal that did fire, in `narration.test.ts`, and
  it fired as an off-by-one on an unrelated pin rather than as a finding.

### The fix, and the general rule

The lesson now carries the `.tex`'s prose and gains a strand the table had no
column for (*sē* is the root of *separate* and, at a distance, *suicide*). It
says outright why the set is named and not gridded: *"you meet each one where it
earns its place."*

Generalised: **a hand-written chapter and its lessons can disagree in BOTH
directions.** The parity gate measures one of them. The other — content the
lessons carry that the hand-written page deliberately withheld — has no
instrument at all, and a chapter flip publishes it.
