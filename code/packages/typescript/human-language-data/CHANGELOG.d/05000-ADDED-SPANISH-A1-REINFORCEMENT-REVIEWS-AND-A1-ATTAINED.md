### Added - Spanish A1 reinforcement reviews, and the first A1 any track has held

- **Spanish attains A1.** `attained` moves `pre-A1 -> A1` and `inProgressAt`
  moves `A1 -> A2`. This is the second level any of the 24 tracks has ever held,
  and criterion 4 (§3.1: every atom at or below the level revisited at least
  twice) was the last of the five to close.
- Author **eighteen `review` lessons** in two tranches. The first eleven carry the
  59 atoms the wiring pass could not reach, taking reinforcement at or below A1
  from **60 -> 0**. The remaining seven close the **10 atoms / 13 slots** that
  `main` reopened underneath this branch while it sat unmerged; see *The rung
  moved while we were standing on it*.

#### The count that mattered was slots, not atoms

The residue was 59 atoms but **72 slots**: an atom at `revisits=0` needs *two*
further passes, not one, and `measureContinuity` counts **distinct later
lessons**, so one review can contribute at most **one** revisit to any atom. A
plan sized against 59 would have left every zero-revisit atom open while looking
complete -- the atoms named, the lessons landed, the number barely moving. The
trap was concentrated in chapters 100-199: 11 atoms in 18 slots, **seven of them
at zero revisits**, which is why that stretch gets two reviews and not one. Both
numbers are reported here for the same reason they were planned against.

| chapters | atoms | slots | zero-revisit |
|---|---|---|---|
| < 100 | 6 | 9 | 3 |
| 100-199 | 11 | 18 | 7 |
| 200-299 | 41 | 44 | 3 |
| 300+ | 1 | 1 | 0 |

#### The lessons are threads, not carriers

A review that revisits nothing is worse than an open atom, so each was built
around something the source lessons already say, and several groupings are the
corpus quoting itself:

- **`ES-C67-repaso-lo-que-reemplazo-los-casos`** -- `veo a María` says Spanish
  "lost the endings that told you who did what to whom, and then quietly grew a
  replacement"; `María, ven` says Latin's vocative case was "replaced with a
  comma"; `hoy como en casa` says "what was lost cost case marking, and what was
  kept bought word order". Three lessons telling one story, plus the article's
  job (`María` / `la señora García`) and position deciding form (`mucha agua` /
  `come mucho`, `el primer libro`).
- **`ES-C67-repaso-cuando-dos-palabras-se-tocan`** -- `primer-libro`'s own hook
  names "the same biting-short that made *muy* out of *mucho*", so the apocopes,
  the two contractions, `o -> u` and `conmigo` (which says *with* twice) are one
  phenomenon: what happens where two words meet.
- **`ES-C44-repaso-ir-ida`** and **`ES-C67-repaso-la-terminacion-y-el-infinitivo`**
  -- the present of *ir* belongs to *vādere*, so *ir* and the `-ida` noun built
  on it are the only places *īre* survives; and the forms that name no person
  (infinitive, `-ida` noun) are the ones that stop behaving like verbs.
- **`ES-C40-repaso-un-sabado-de-octubre`** pairs `ES-CULTURE-ROMAN-MONTH-NAMES`
  (chapter 47) with `ES-LEX-MONTHS-01` (chapter 145) -- the same fact taught 98
  chapters apart, and *octubre* is the worked example.

Roughly a third of the candidate groupings were rejected as the detector
matching rather than the teaching, on the standard the wiring pass set.

#### Insert mid-track -- and, in exactly one place, append

The first tranche landed every lesson inside an **existing** chapter, at chapters
81 through 345 and never past 345, on this reasoning: appending would have made
`ES-LEX-BORRAR` and `ES-LEX-GRITAR` measurable and **manufactured** two blockers
while closing others.

That reasoning was right when it was written and is now spent, because `main`
made `ES-LEX-GRITAR` measurable without asking. The rule it was protecting --
never add a lesson *in order to* make a window measurable -- is still held: the
appended pair discharges a blocker that already existed rather than creating the
one it then closes.

The append is bounded, and the bound was verified rather than assumed. The two
lessons take the track's last position to 1016, and the windows that opens are
exactly the windows they close:

| atom | position | window opened | closed by |
|---|---|---|---|
| `ES-LEX-EMPUJAR` | 1010 | R2 at 1015 | the first appended review |
| `ES-LEX-LANZAR` | 1011 | R2 at 1016 | the second |
| `ES-LEX-GRITAR` | 1013 | R1 at 1014 | both -- two revisits, which is the point |
| `ES-LEX-SABER` | 1014 | R1 at 1015 | the second |

`ES-LEX-BORRAR` at 1012 is the one that stays out of range: its R2 opens at 1017
and the track ends at 1016. One more lesson after these and it becomes real --
**that** is the edge the next tranche now inherits, one position further along
than the edge this PR originally declared.

#### Registration is three files

A `review` placed in an existing chapter needs the lesson, its `curriculum.d/path`
segment and its `curriculum.d/extensions` entry. No `chapters.d` entry and no
`book.tex` line, because chapters do not enumerate lessons. Track-local
`concept_tag`s need no `concepts/taxonomy.json` entry, and `type: review` with
headword `(review)` is not a `CONTENT_TYPE`, so none of this moves a vocabulary
count.

#### The rung moved while we were standing on it

This branch was authored against a corpus where the A1 reinforcement residue was
59 atoms. Between then and merge, `main` moved twice:

- **#13132** and **#13144** brought the first four DELE A1 verbs, and then
  `tener`/`poder`/`saber`, **down from A2 into A1**. Their atoms came with their
  own reinforcement debt, and it landed at A1 because that is where the verbs
  now live. Nine of the ten new blockers arrived this way -- five subjunctive
  singulars at `revisits=1`, `ES-LEX-ANDAR-08`, `ES-LEX-DE-PIE-11`,
  `ES-GRAMMAR-PORQUE-THREE-05`, and `ES-LEX-HABITACION` with `ES-SEMANTIC-LODGING`
  at `revisits=0`.
- The corpus **grew past chapter 388**, so `ES-LEX-GRITAR` stopped being last and
  its R1 window became judgeable -- the tenth blocker, and precisely the
  known-open edge this entry had declared.

Nothing about the first tranche was wrong. A level claim is a statement about a
corpus, and the corpus is not frozen while a branch waits. The lesson worth
keeping is that **criterion 4 is re-opened by any PR that moves material INTO a
level**, not only by PRs that add lessons to it -- moving a verb down a rung
carries its debt down with it.

Counted in slots rather than atoms, the second tranche is 13: seven atoms needing
one more revisit, three needing two.

#### Pins, re-measured from a from-scratch `dist/`

**304** headwords at or below pre-A1 against a floor of 300; **638** headwords and
**58** verb headwords at or below A1 against 600 and 40.

Not one of those moved because of this branch. `type: review` with headword
`(review)` is not a `CONTENT_TYPE`, so eighteen review lessons contribute exactly
zero headwords -- pre-A1 is unchanged at 304, and A1's rise from the 617/40 this
entry first recorded to 638/58 is #13132 and #13144 moving verbs down a rung.
A1 verb vocabulary was met **exactly** at 40 when this branch was written; it now
clears the floor by eighteen, which is margin this branch did not earn and should
not claim.

Because `vocabularyOf` and `verbVocabularyOf` are module-private, the counts were
re-derived by mirroring them, and the mirror is **self-checked** against the two
figures the gate still prints for itself (**672** and **78** at A2) -- a mirror
that cannot reproduce those is not evidence about anything.

#### Tests

- **`level-gate.test.ts` inverted, and de-pinned while inverting.** Four
  assertions encoded "Spanish sits at pre-A1 with a reinforcement blocker". The
  one the brief named asserted `attained !== "A1"` and that `blockers` contained
  `reinforcement`; both have expired, and the second would now be *false about a
  different rung*, since `blockers` scopes to the first failing level and that is
  A2, where `vocabulary` and `verb-vocabulary` are legitimately open again. Each
  is now a **floor or a relation** rather than a literal -- `attained >= A1`,
  `inProgressAt > attained`, the attained rung read off the gate rather than
  hard-coded -- so the next tranche to climb a rung does not have to edit this
  file. Criterion 2b is re-derived at A1 from the corpus instead of read off a
  list that moved.
- **A transition fixture, because closing the criterion removed its only proof.**
  `level-gate-transition.test.ts` builds a synthetic track meeting every other
  criterion and **one revisit short**, asserts `reinforcement(-1)` is its sole
  blocker, then adds exactly that revisit and asserts the level is attained --
  with a third test proving the two runs differ at one array index and in one
  field. Twenty other tracks still fail `reinforcement`, so the corpus keeps red
  cases; none of them proves the *transition*. Verified to bite: with
  `revisits < 2` weakened to `< 1` the blocked half fails.
- The fixture also records two facts about the gate that are easy to get wrong:
  criterion 4 is a **count of revisits, not a window check**, so a defect can
  persist across the fix while the verdict flips; and an atom in a track too
  short to contain even R1 produces no defect at all and is invisible to the
  criterion.

#### No ceiling raised, and one cut

`ES-C67-repaso-lo-que-reemplazo-los-casos` first measured **316 effective
seconds** against the 300-second model and was **cut** to 288, not re-declared.
`ruleStatements`, the banned-word ceiling and the paradigm-table counts are all
unchanged; no `sounds` tag, glyph or block title is new. All generated artifacts
regenerated, including the `spanish/curriculum.json` monolith.

#### Fixed while here

`ES-C345-repaso-persona-objeto` (the pilot review) said "this course" in its
`gloss` and `etymology_hook`, which reach the generated `.tex` and trip
`standalone-book`'s cross-volume claim scan. Both rewritten. The lesson had
passed the 18-test corpus validator; only the full suite caught it.
