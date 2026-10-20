## Seven chapters on the case system the book promised: 37/70 → 56/70, and five columns close

`GE-C14-einen` taught *einen* as a bare word and said so on the page:

> Take this one as a word for now, not as a system. The system behind it —
> German's cases — gets a chapter of its own later; this is the first place the
> book cannot avoid it.

**That chapter had never been written.** Chapters 45–51 are it, and six more
points came along with it because the same ending set turns out to run under
*kein*, *mein*, *dein*, *sein*, *ihr* and *ihn*.

### Measured with `measureExamCoverage`, before and after, on the merged tree

| | before | after |
|---|---|---|
| A1 exam coverage | 37/70 (53%) | **56/70 (80%)** |
| lessons | 297 | 339 |
| atoms taught | 468 | 512 |
| atoms never revisited | 80 | **51** |
| reinforcement window misses | 1015 | **955** |
| forward references | 25 | 30 |
| lessons over the computed 300s ceiling | 0 | 0 |
| atom lesson spikes / chapter spikes | 8 / 5 | 8 / 5 |
| cross-chapter prose references | 0 | 0 |
| LaTeX warnings (all six kinds) | 0 | 0 |

Re-measured on the tree, never derived from a delta.

### Five columns go to full

| column | before | after |
|---|---|---|
| Das Verb | 7/12 | **12/12** |
| Das Pronomen | 3/5 | **5/5** |
| Der Satz | 2/5 | **5/5** |
| Die Frage | 0/4 | **4/4** |
| Die Negation | 1/3 | **3/3** |
| Der Artikel | 2/5 | 4/5 |

### The ratio comes from spending, not minting

Thirty-five items close nineteen points, and the reason is one ending set bought
four times over:

- ***kein*** is ***ein*** with two letters welded on, so it inherits the endings
  the *ein/eine* lesson already paid for. Two words close **two** points
  (`A1-ART-03` and `A1-NEG-02`).
- ***keinen*** is ***einen*** with the same two letters. The reader has been
  saying that `-en` since the *haben* chapter without being told what it was.
- ***den*** then names the row all of them move on — and that is the whole of
  the German case system at A1: **one row of three moves, and it is the *der*
  row.**
- ***mein, dein, sein, ihr*** wear the same endings a fourth time, so four
  possessives cost their first two letters and nothing else.
- ***ihn*** is the sixth word to move on that row.

`GE-GRAMMAR-NATIVE-VS-LOAN-01` and the Grimm's-law atoms are spent the same way:
*wer/who*, *wann/when*, *warum/wherefore*, *viel/full*, *dein/thine* and
*Fuß/pes* are all one sound law applied to a new word rather than a new fact.

### What is taught, one item per lesson

| | |
|---|---|
| **Ch. 45** *kein, keine, keinen*, the choice, the placement | *ni* + *ein* is "not one", exactly as *ne* + *ān* gave English **none** |
| **Ch. 46** the polar question, the imperative, *wer, wann, warum* | German asks with **w-** where English asks with **wh-**; five words, one law |
| **Ch. 47** *welcher*, *wie viel/viele*, *den*, the bare adjective, *es gibt* | the chapter `GE-C14-einen` promised |
| **Ch. 48** *oder, aber, sondern, denn, weil* | *weil* is **die Weile**, English *while* — a time word promoted to a cause, as English promoted *since* |
| **Ch. 49** *mein, dein, sein, ihr, das ist* | six words on one ending set |
| **Ch. 50** *dich, ihn, uns, euch, sich* | *euch* is Old English *ēow* — the object pronoun that became **you** |
| **Ch. 51** *können, müssen, möchten*, the bracket, the *-en* participle | four things go to the end of a German sentence, and the reader had met three of them |

### Zero reinforcement debt created, and sixty paid

Every one of the 44 new atoms is revisited, and **not one misses a window it was
long enough to have**:

| window | misses created by this tranche |
|---|---|
| R1 (1–3 later) | 0 |
| R2 (5–15) | 0 |
| R3 (20–60) | 0 |
| R4 (80–250) | 0 |

That is arithmetic, not luck. Each chapter opener retrieves the two preceding
items by name and carries a `[YOU RECALL: …]` for the item two chapters back;
each chapter's review pays the R2 slots of the chapter before it, and the later
reviews pay the R3 slots of the earlier chapters on a diagonal. Five payments
were added after the first measurement, each to a lesson that lands inside the
window it closes.

Meanwhile the 42 added lessons **newly expose** windows that the old tail of the
track was too short to have: an atom taught in chapter 44 had nothing after it,
so R1, R2, R3 and R4 were all uncounted for it. The tranche pays those as it
goes, which is where the falls come from:

| | before | after | of which this tranche's own |
|---|---|---|---|
| atoms never revisited | 80 | 51 | 0 |
| R1 misses | 152 | 152 | 0 |
| R2 misses | 268 | 258 | 0 |
| R3 misses | 341 | 319 | 0 |
| R4 misses | 254 | 224 | 0 |

**Twenty-nine atoms stopped being orphans**, and sixty window misses were closed,
by lessons whose job was something else entirely.

### The one number that rose, and why

`forwardReferences` goes 25 → 30, and **all five are pre-existing debt made
legible**. The corpus was already using *mein*, *wann* and *aber* in earlier
lessons without teaching them anywhere; now that each has an owner, those uses
read as forward references instead of as nothing at all. A word used and never
taught scored zero before this tranche; a word used before it is taught scores
five. The second number is the honest one.

An earlier draft pushed this to 92, and that WAS real damage: `das ist` as a
headword strips its leading article and leaves **ist**, which made every earlier
lesson in the track a forward reference to chapter 49. The headword is now `Das
ist mein Bruder` and the count is back to five.

### What moved that the coverage total cannot see

- `A1-SATZ-02` was already covered on the separable prefix and its probe now also
  names `GE-GRAMMAR-VERBKLAMMER-01`. **The total does not move**, because a
  covered point cannot become more covered — but the corpus went from
  *demonstrating* the verb bracket to *naming* it, and naming the separable
  prefix, the participle and the cancelling *nicht* as three earlier instances of
  the same habit.
- `GE-C34-praedikativ` states a rule the book has shown on page one since it was
  written: *guten Tag* inflects and *der Tag ist gut* does not. Both halves were
  always on the page and no lesson had ever put them side by side.
- `GE-C37-sich` explains **freut mich**, said since the second chapter as an
  unanalysed block. It was a reflexive all along.
- Five spine-omission ledgers go empty — `VERB-NEGATE`, `QUESTION-POLAR`,
  `CONNECTIVE-BECAUSE`, `VERB-CAN` and `VERB-WANT` — and none of that is exam
  coverage either.

### What is left, and why

`Die Praeposition` is the one column untouched, at 0/4, and it is deliberately
next rather than now: `A1-ART-05` (dative article forms) sits **underneath**
`A1-PRAEP-02`, so *dem* and *der* have to exist before a dative preposition can
be shown governing one — and teaching the seven dative prepositions costs seven
items for one point until that article exists. The remaining nine open points
are the tens above twenty, the half-hours, the countries, the plural patterns,
the attributive adjective endings, three adverb classes and word stress.

German's 70 points restate the Goethe-Zertifikat A1 structure rather than the
Spanish/PCIC proxy the twenty derived tracks use, so a point here is worth
proportionally much more and the percentage is not comparable with theirs.

