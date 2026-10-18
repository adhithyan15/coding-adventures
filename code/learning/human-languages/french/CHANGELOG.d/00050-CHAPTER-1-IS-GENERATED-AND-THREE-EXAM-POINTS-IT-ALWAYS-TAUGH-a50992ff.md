## Chapter 1 is generated, and three exam points it always taught are finally visible

Chapter 1 — *Greetings*, the first thing a reader of this book ever meets — was
hand-written LaTeX over eleven schema-v1 lessons and four schema-v2 writing
lessons. It is now **fifteen** schema-v2 lessons and generated from them.

### Four ways of sizing it, and what each one said

| instrument | answer |
| --- | --- |
| `handwritten_parity.py` | **0** — the chapter does not even appear in the report |
| `grep -l '^chapter: 1$' lessons/*.md` | **15** lessons (11 content, 4 writing) |
| the `.tex` | **15** sections |
| taught-word census | **0** French words owned by no lesson |

The census's residue — 98 distinct emphasised tokens in the `.tex`, of which 52
match no French headword — is entirely Latin etymons (*diurnum*, *noctem*,
*ille / illa / illos*), Spanish comparanda (*hola*, *bueno / buena*, *tarde*),
pronunciation respellings (*zhoor*, *swahr*, *nwee*), morphological fragments
(*-ct-*, *-it-*, *nn*) and inflected forms of words the chapter already owns
(*le jour*, *les soirs*, *bonnes*). Not one unowned French headword.

For once every instrument agreed, and the agreement was real: chapter 1's
hand-written `.tex` mirrors its lessons section for section, including the
Latin/Spanish/French/English cognate table under *nuit* and the three-line
agreement display under *bonne nuit*. **Nothing had to be written.** The whole
job was typing what was already there, which is why this is the only French
retirement so far with a net lesson change of zero.

That is the exception rather than the rule, and it is worth naming why: chapter 1
is the one chapter whose lessons were authored *before* the LaTeX rather than
extracted from it.

### It lands at exactly twelve atoms, and did not split

`maxNewAtomsPerChapter` is 12 and `atomChapterSpikes` for French is 0, so the
question was real. Ten words and phrases (*salut*, *bien*, *bon*, *jour*,
*bonjour*, *soir*, *bonsoir*, *nuit*, *bonne nuit*), the two grammar rules the
greetings run on, and the writing runway's `FR-ORTHO-SALUT-01` come to **twelve
exactly**. Chapters 3, 6 and 7 sit at the same ceiling.

The etymology stays prose. *salus*, *bene*, *bonus*, *ille*, *diurnum*, *sērum*
and *noctem* are all still taken apart on the page, and none of them is a typed
atom — the same call chapter 7 made, where only three of eight day-names earned
an `FR-ETYMON-*`. Typing all seven here would have been sixteen atoms and a
split, and a split of chapter 1 renumbers every chapter in the book.

### Two grammar rules that were being taught and could not be measured

- **`FR-GRAM-ADJ-AGREEMENT-04`** — *bon*, *bonne*, *bons*, *bonnes*, introduced
  in the *bon* lesson and then spent across three: *bonjour* and *bonsoir* take
  the masculine, *bonne nuit* the feminine. The rule arrives with the words that
  need it rather than as a table.
- **`FR-GRAM-LE-LA-GENDER-05`** — *le* / *la* / *les* as one atom, because they
  are one system: the article is where a French noun's gender is visible, and
  *les* is the single form both genders share.

### It closes three exam points, and none of them needed new content

French A1 coverage **27/74 → 30/74**: A1-LEX-01 (greetings and farewells),
A1-D-01 (the definite article) and A1-A-01 (adjective agreement). All three were
on page one from the beginning; none could be probed, because a hand-written
chapter's `grammarlens` owns no atom. A1-LEX-01 had the farewells already —
chapter 4 is generated — so half the point existed while the whole point read as
absent. Recorded in `BACKLOG.d` as HL-C312, because it applies to every
hand-written chapter that remains.

### The path was out of order, and a cross-reference rotted

Two repairs the gates found:

- `FR-PATH-005` (*bonne nuit*) sat **before** `FR-PATH-006` (*soir*, *bonsoir*)
  in the curriculum path, while its lesson sequence is later. Harmless while the
  lessons were schema-v1 and unenforceable; a hard prerequisite-order failure the
  moment they were typed. The two segments are swapped, which is also the order
  the chapter is read in.
- `ch02-introductions.tex` pointed at `\S\ref{lesson:le-la}`, a label the
  hand-written chapter 1 defined and the generated one does not. Rewritten to
  name the thing — "the *le* / *la* lesson" — which is the rule
  `chapter-references.test.ts` already holds French to for chapter numbers.

### One defect only the page showed

The delayed-copy writing runway authored its four steps as a markdown ordered
list. The generator emits `itemize` for `-` bullets and **nothing** for `1.`, so
the four steps printed as one run-on line where the hand-written chapter had a
real `enumerate`. Rewritten as bullets. 169 lessons across the corpus have the
same shape and every generated chapter among them is already shipping it; the
general fix is a `src/book.ts` change and wants its own commit.

### Measured, against the merged tree

- Hand-written French chapters: **10 -> 9**; corpus **21 -> 20**.
- `handwritten_parity.py french`: **45** blocks at risk, unchanged — chapter 1
  contributed none of them.
- Schema-v2 French lessons: **108 -> 119**.
- Atoms taught: **190 -> 201**.
- Atom-measurement-blind lessons: **39 -> 28**.
- Chapters over the 12-atom budget: **0 -> 0**.
- Culture claims: **17 -> 20** — *salut* is strictly informal, *bonjour* is
  near-obligatory on entering anywhere, *bonne nuit* is for bed and not for the
  evening. All three were prose the `.tex` boxed as `culture` and no lesson owned.
- Forward references: **48 -> 48**.
- Reinforcement window misses: **416 -> 443**. Eleven new atoms exist to be
  missed; every one of them is revisited at least once, so `atomsNeverRevisited`
  holds at **37**.
- Cross-chapter prose references: **0 -> 0**.
- Completion-plan uncovered points: **851 -> 848** when this branch was cut,
  re-measured by running the CLI rather than by subtracting three. HL-C310
  landed on `main` in the meantime and removed that absolute pin from
  `plan-cli.test.ts` altogether, for exactly the reason this entry keeps
  re-measuring rather than composing: the figure moved nine times in one day and
  two branches that both lower it merge quietly because they agree. The test now
  asserts the invariant it actually owns — that duplicating an inventory changes
  neither figure — and the corpus total after this merge is **785**.
- The book compiles under XeLaTeX with `missing_character = 0` and every warning
  class at its baseline, and the rendered pages were read.
