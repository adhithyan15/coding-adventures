## HL-C396 — twenty-two tracks never declared their grammar cells

Found while scoping Marathi's `MR-A1-V-01`, *the present habitual, all persons*.
It stopped that chapter from being written the wrong way, and it applies to
every track but one.

### The policy

`core/chapter-policy.json` sets **`maxNewGrammarCellsPerLesson: 1`**, and
`data/generate_grammar_cells.py` states what a cell is:

> *"A **cell** is one filled slot in one paradigm: Spanish `hablo` is a cell.
> `PRESENT-INDICATIVE-CONJ1` — the whole six-form table — is **not** a teachable
> unit; it is six."*

And the rule has a second half that is easy to miss: **no paradigm table until
every cell in it has been taught individually.**

The policy's own rationale is unusually direct about what that costs, and it is
worth quoting because it is a sizing decision rather than a limit:

> *"Spanish holds roughly **630 verb cells**. At one per lesson the verb system
> alone is **~630 lessons**, which is the honest size of the thing rather than a
> number to optimise away."*

### The hole

`data/generate_grammar_cells.py` describes a two-file contract — a
language-neutral slot inventory and each track's filling — and says a track
lacking a slot **"declares an omission rather than leaving a hole, the same
contract `curriculum.json` already uses for spine nodes."**

| file | exists |
|---|---|
| `core/grammar-slots.json` | yes |
| `spanish/grammar-cells.json` | yes |
| every other track | **no** |

**One track of twenty-three has declared its cells.** The other twenty-two have
neither a filling nor a declared omission — they have the hole the contract was
written to prevent.

The docstring anticipates the consequence: the one-cell rule *"is only
enforceable if the cells are enumerated somewhere."* Where nothing is
enumerated, nothing is enforced.

This is **not a broken build**. All seven of these budgets ship **report-only**,
by the HL05 precedent, *"because the corpus predates them and a gate that fails
on recorded debt teaches authors to route around it."* So this is unrecorded
debt rather than a regression, which is exactly what this file is for.

### Why it matters right now

`MR-A1-V-01` asks for the present habitual **in all persons**. Marathi is
missing at minimum the 2SG masculine, 2SG feminine, 1PL and 3PL cells — and
that is for **one verb**. Under the policy that is four lessons at one cell
each, and the six-form table may not be printed until all six have been taught.

A first draft of that chapter was going to be four lessons **with the paradigm
table in the recall**. The policy forbids exactly that, and `HL-C395`'s
suggested order should be read with this constraint attached.

There is a second reason to care about the Marathi case specifically: the
present tense there is **syncretic**. मी येतो, तो येतो and आम्ही येतो are one
form serving three cells. A cell inventory is what makes that statable — three
slots, one filling — where a paradigm table would just show the form once and
hide the fact.

### What the fix looks like

Not a curriculum tranche. **Author `marathi/grammar-cells.json` first**, against
the slots `core/grammar-slots.json` already names, declaring omissions where
Marathi has no counterpart (it has no articles, and its past is ergative, which
the Spanish-derived slots will not name). Then the verb campaign has something
to ramp against and to be measured by.

The same is true of the other twenty-one. Spanish's file is the worked example.
