---
category: Repo policy / workflow reminders
---

# Folding a wide table to three columns must keep a paradigm's person column, or the info-dump gate stops seeing the grid

**What went wrong.** Nineteen four-column tables that opened chapters were
folded to three labelled columns, so the narrator reads them and the chapters can
be started in the car. The usual fold drops a derivable column. For FR-C05-parler,
the draft dropped `person`, because *je parle* already names the person. Every
`check:*` gate passed. The full suite then failed `tests/info-dump.test.ts`:
`fullParadigmGrids` went from 20 to 19, and the named fixture
`grids.has("FR-C05-parler")` was false.

`info-dump.ts` finds a paradigm by counting person labels (`je`, `tu`, `yo` …)
in a table's FIRST column. Without that column, the six-form grid was still on
the page, but the gate could no longer see it. That is the same as dodging a
detector, even though the narration count improved.

**The fix.** Keep the person column and fold the ending into the form instead:
`| person | ending, on *parler* | sounds like |`, with cells such as
`**-e**: **je parle**`. The table is still three columns, the narrator reads it,
and the grid is still flagged.

**Do differently.** Before you fold a table, check what else reads its shape.
`info-dump` reads the first column for persons. `chapter-policy` treats an
unlabelled first column as unspeakable. Fold away columns that only a person
reading the page needs, and keep the column a gate keys on. Then run
`tests/info-dump.test.ts` and `tests/narration.test.ts` together.
