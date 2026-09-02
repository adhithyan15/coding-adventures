## HL-C286 — read the tables too, and what French chapter 18 will cost

French chapters 6, 7, 8 and 9 are now generated, chapter 9 having become three.
**Nine hand-written French chapters remain.** This entry adds the clause the
earlier sizing note was missing, and hands off the one landmine the remaining
nine contain.

### The sizing rule has three clauses, not two

An earlier entry recorded two ways to size a hand-written chapter: the parity
gap counts LaTeX environments, and counting the French words the `.tex` teaches
against the lessons that own them finds the vocabulary hole the gap cannot see.

**Both can pass while the chapter is still eight lessons deep**, because a
chapter can be short of *grammar* rather than short of *words*. German chapter 5
went 5 → 15 lessons on exactly that: one lesson introduced a verb, a six-row
present-tense grid and five untaught pronouns in a single sitting. The word
count was fine. The grid was not.

So the check is four questions, and the fourth is not a grep:

1. `handwritten_parity.py` — how many prose blocks would vanish.
2. Words the `.tex` teaches, against the lessons that own them.
3. `grep -l '^chapter: N$'` — how many lessons the chapter actually has.
4. **Read the chapter's TABLES.** A table with one row per grammatical person is
   a **paradigm**, `maxNewGrammarCellsPerLesson` is **1**, and a six-row grid is
   therefore six lessons' worth of cells in one. No grep will tell you this;
   `info-dump.ts` will, and only after the lessons exist.

Chapters 6–11 were checked against clause 4 after the fact and introduce **zero**
paradigm tables and zero info-dump findings — the one-word-per-lesson shape gets
this right for free. A chapter whose subject *is* a paradigm will not.

### The landmine: FR-C16-passe-compose-etre

French holds exactly **two** full paradigm grids:

- **`FR-C05-parler`** — chapter 5, already generated, **not** in the retirement
  scope. It is also a **named fixture** in `info-dump.test.ts`
  (`expect(grids.has("FR-C05-parler")).toBe(true)`), and since German inverted
  its own fixture it may now be the only one left firing positive. **Do not
  touch chapter 5 while retiring hand-written chapters.** There is no reason to:
  it is already generated.
- **`FR-C16-passe-compose-etre`** — in a hand-written chapter, **squarely in
  scope**, and it is the *être* paradigm plus the verb list that selects it.

When that chapter is authored, its grid disappears and
`report.summary.fullParadigmGrids` **falls**. That fall is the gate working, not
a regression to route around. The German precedent is the procedure:

- **Do not delete the assertion.** Lower the pin with the reason written beside
  it.
- If the lesson is a **named fixture**, **invert** it to `toBe(false)` rather
  than removing the line, so a regression still fails loudly.
- **Re-measure against the merged tree.** Do not compute *previous − 1*. That
  arithmetic has been wrong repeatedly: this branch alone watched the
  uncovered-point total go 529 → 686 → 793 while it was open, as Telugu and then
  Tamil inventories landed independently. Every number in this PR was taken from
  `npm run plan` **after** the rebase, never derived.

### Two operational traps this tranche hit twice each

**A renumber leaves stale generated shards, and the generator does not prune
them.** Moving French chapters 10–33 to 12–35 left book-hash shards for chapters
that had become hand-written. `check:books` said only "missing, stale, or
malformed"; it was **`check:shards`** that named them, as a cross-ledger identity
mismatch. It happened a second time when a rebase restored one of them, because
`git checkout --theirs` **fails silently for a file deleted on your side** and
the conflicted copy survives. After any renumber, diff the hash set against the
ledger set directly:

```
ls core/generated-book-hashes/<track>.d/ | ...   # against
ls core/book-generation.d/targets.d/<track>-*    # never handwritten.d
```

**`git add` on a rebase-resolved path stages conflict markers.** Resolving 60
generated files by `checkout --theirs` and then `git add`-ing the directory
staged **60 files still containing `<<<<<<<`**, and the rebase reported success.
Generated files must be resolved by **regenerating**, never by taking a side —
and `git ls-files -z | xargs -0 grep -lE '^(<<<<<<< |>>>>>>> )'` belongs before
every commit, not only after a merge that looked hard.

### Forward references move both ways, and both are correct

German's fell three chapters running, 42 → 39 → 35, as teaching the missing
words retired previews that earlier chapters had been making. French's **rose**
48 → 53 when the months chapter landed, for the mirror-image reason: **the gate
cannot see a forward reference to a word no lesson teaches**, so teaching the
twelve months made chapter 6's previews of them visible for the first time.

Five of those were decorative and were removed. Three were kept —
*sept* → **septembre**, *huit* → **octobre**, *neuf* → **novembre** — because
they are the whole didactic move of that chapter and the point cannot be made
without naming the month. **Read the number; do not optimise it.**
