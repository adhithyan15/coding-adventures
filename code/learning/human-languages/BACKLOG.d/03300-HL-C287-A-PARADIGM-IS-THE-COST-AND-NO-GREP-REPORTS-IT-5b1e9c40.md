## HL-C287 — a paradigm is the cost, and no grep reports it

German chapter 16 passed every sizing instrument the retirement had:

  * `handwritten_parity.py` scored it at **8 blocks**;
  * `grep -l '^chapter: 16$' lessons/*.md` — the honest denominator that caught
    chapter 4's three hidden writing lessons — said **3**;
  * the `.tex` rendered **2 sections**;
  * the taught-word census gave **six** new German words (*sein*, *müde*,
    *kommen*, *fahren*, *werden*, *bleiben*).

The chapter became **twenty-six lessons across three chapters**. Every
instrument above was looking at the wrong axis, because chapter 16's cost is
**grammar, not vocabulary**, and grammar arrives in a shape none of them counts:
a table with one row per person.

**A cell is one filled slot, not a table.** `maxNewGrammarCellsPerLesson` is 1.
The present of *sein* is six cells and its past is four, so the two grids the
`.tex` printed in nine lines of LaTeX are **ten lessons**. Add six lexical,
sound and rule atoms and the chapter is twenty-four atoms against a
`maxNewAtomsPerChapter` of twelve — so it is three chapters, not one dense one.

**The fourth sizing clause, and where it applies.** Read the chapter's TABLES,
and count a person-labelled first column as a paradigm rather than a table.
This is not a German quirk: the higher chapters of every track are where the
paradigms cluster, because that is where the verb system arrives. French 16
(*être*) is the same chapter with the same shape and is still hand-written.

**Two things the split costs that a one-chapter flip does not.**

1. **Fifteen chapters renumber, and the prose that names them rots on that one
   commit.** German had 65 cross-chapter prose references and **36 pointed into
   the renumbered range**. `chapter-references.test.ts` had already written the
   answer for Spanish and French: name the thing, never the number, and clear
   the track BEFORE the first split rather than after the first rot. German is
   now pinned at zero beside them. The README and roadmap need the same pass —
   96 chapter pointers between them — and there the fix IS a fresher number,
   because those documents are a map of the book rather than teaching prose.
2. **Adding path shards renumbers every later one.** `shardFilenameFor` names
   shards `(index + 1) * 10`, so inserting two segments after `GE-PATH-021`
   renamed twenty-one files. `check:shards` catches it, but it is a large
   rename to discover at verification time rather than plan for.

**Where the paradigm grid goes.** HL10 forbids printing a paradigm table until
every cell in it has been taught individually, at which point the table is a
recap. That rule has a pleasant consequence a per-cell split makes available:
by the time chapter 18 prints *ich bin gegangen / du bist gegangen / …*, the
table introduces **nothing** — every cell of *sein*'s present is owned and the
participle does not inflect — so a four-row paradigm-shaped table is honestly a
review. `info-dump.ts` cannot tell the two apart and will report it either way;
the check that it is a recap is the reading, not the gate.

**And a false positive worth knowing about.** `info-dump.ts` reads a table with
three or more person-labelled first cells as a paradigm. A German dialogue whose
lines open *Ich…*, *Du…*, *Wir…* is exactly that shape and is not a paradigm at
all. Two practice dialogues tripped it here. Opening those lines with something
other than a pronoun — *Danke, ich bin müde*; *Dann sind wir müde* — is better
dialogue anyway, and it keeps the corpus paradigm count honest.
