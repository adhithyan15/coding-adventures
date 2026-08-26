# HL10 — The Spanish pre-A1 → C2 course: complete architecture

**Status:** specification, 2026-08-10
**Pilot track:** Spanish. Every structure here is designed to be lifted into the
other 21 tracks; §4 states exactly which parts are universal and which are
Spanish-local.
**Builds on:** HL09 (the ramp's *shape*), HL08 (modality and the drivable course),
HL05 (chapter capability), HL04 (the shared spine), HL01 (the concept taxonomy),
HL00 (the lesson schema).
**What it adds:** HL09 says how gentle a step must be. It does not say what the
steps *are*. This spec is the course itself — the strands, the spine, the grammar
lattice, the etymological system, the stage-by-stage map, and the practice model
that carries a learner from no Spanish at all to C2.

---

## 1. What this is, and the one sentence it exists to satisfy

> Build a Spanish course so gentle that no step is ever hard, so long that
> gentleness never costs coverage, and so connected that every lesson is paid
> for by a later one — from the first word to reading Cervantes.

Three of those are already policy. HL09 fixed step size (≤3 new atoms), HL00 fixed
lesson length (<300 seconds), HL08 fixed the drivability contract. What has never
existed is the **course architecture**: the thing that decides that `gracias`
comes before `hola`, that the preterite takes eleven chapters and not one, that
`por`/`para` is fourteen lessons and never a table, and that the Arabic layer of
Spanish arrives at A2 rather than as a C1 curiosity.

### 1.1 The four constraints this design is under

| constraint | source | consequence for this design |
|---|---|---|
| ≤5 minutes per lesson | HL00, enforced, zero violations today | The lesson is the atom of design. Everything is expressed as *how many lessons*, never *how long a lesson*. |
| ≤3 new atoms per lesson, ≤12 per chapter | `core/chapter-policy.json` | Coverage can only be bought with lesson count. Hence §11's arithmetic. |
| Length is free — 10,000 pages is acceptable | owner directive, 2026-08-07 | Splitting is always the correct answer to steepness. No gate may reward brevity. |
| Never info dump | owner directive, 2026-08-10 | §7.3 makes this a measured gate rather than an aspiration. |

### 1.2 What this spec deliberately does not do

- It does not author lessons. It says what must be authored, in what order, and
  what would make each one wrong.
- It does not restate HL09. Where HL09 already fixes a rule (the R1–R4
  reinforcement windows, the S0–S12 sentence rungs, the vocabulary selection
  order), this spec cites it and moves on.
- It does not touch the other 21 tracks. It is built so they can follow, and §4.4
  states the contract that keeps them able to.

---

## 2. The central architectural move: eight strands

Today the curriculum has one organising line — the FUNCTION spine, "I can greet
someone" — and everything else (grammar, sound, etymology, culture) rides along
inside whichever lesson happens to need it. That works for 188 lessons. It
collapses at 5,000, because there is no way to ask "is the grammar ramp gentle?"
when grammar is not a thing the data model knows about.

So the course runs **eight parallel strands**. Each is a ladder in its own right,
each has its own budget, each ramps independently, and every chapter draws on
FUNCTION plus at least two others.

| # | strand | what it ladders | why it must be separate |
|---|---|---|---|
| 1 | **FUNCTION** | what the learner can *do* | The existing spine. Universal across tracks. |
| 2 | **GRAMMAR** | the structural system, cell by cell | §5. The steepest ramp in any language course, and the one currently invisible. |
| 3 | **LEXICON** | vocabulary by semantic field | Needs its own ordering (frequency × usefulness), independent of function. |
| 4 | **SOUND** | phonology, orthography, accentuation, regional phonetics | Spanish is nearly phonetic, so this ramp is *short* — which is a gift to spend elsewhere. |
| 5 | **ETYMOLOGY** | roots, layers, productive morphology, and **friends** | §6. The project's signature; at B1+ the engine that makes C2 vocabulary reachable, and via §6.7 the reason a beginner is never starting from zero. |
| 6 | **CULTURE** | history, geography, region, pragmatics, taboo | §7.1. Currently **one** distinct atom in the whole Spanish track. |
| 7 | **IDIOM** | fixed expressions, refranes, collocations, discourse markers | §7.2. Opaque by nature, so it needs the strictest admission rule in the course. |
| 8 | **TEXT** | utterance → turn → conversation → narrative → argument → literary | The C1/C2 payload. Nothing above B1 is really about words. |

### 2.1 Why strands rather than more chapters

A chapter is a *place*. A strand is a *commitment*. The difference matters at the
gate: "chapter 214 mentions voseo" is unfalsifiable as coverage, while "the
CULTURE strand has 340 nodes and every one is realized" is checkable, and its
absence is visible the moment it stops advancing.

Strands also solve the info-dump problem structurally. A lesson that would have
been *"por vs para: fourteen uses"* becomes fourteen GRAMMAR-strand nodes spread
across the stage, each one attached to a FUNCTION the learner needs that day.
The material is not reduced; it is **distributed**.

### 2.2 The strand budget

Each strand gets a per-lesson ceiling, so no lesson can be gentle on vocabulary
while being brutal on grammar — the exact failure HL09 §2.1 diagnosed.

```jsonc
// additions to core/chapter-policy.json
{
  "maxNewGrammarCellsPerLesson": 1,   // §5.2 — the single most important new number
  "maxNewIdiomsPerLesson": 1,         // §7.2
  "maxNewSensesPerLesson": 1,         // §5.5 — polysemy is not vocabulary
  "maxNewCultureClaimsPerLesson": 2,  // §7.1
  "maxRuleStatementsPerLesson": 1,    // §7.3 — the info-dump gate
  "minDownstreamReach": 1,            // §8.2 — no dead-end lessons
  "rootLedgerMinReuse": 3             // §6.2 — an etymon must be cashed in
}
```

A lesson may spend from several strands at once — `comeré` spends 1 grammar cell
and 0 lexicon — but it may never exceed any single ceiling, and the existing
`maxNewAtomsPerLesson: 3` continues to bound the total.

### 2.3 Strand density per chapter

Every chapter must draw FUNCTION + ≥2 other strands, and across any window of ten
consecutive chapters **every one of the eight strands must advance at least once**.
This is what stops the course becoming a grammar syllabus with anecdotes, which
is the default failure mode of every comprehensive language book ever written.

---

## 3. The spine, rebuilt

### 3.1 What is wrong with the spine today

`core/spine.json` has 33 nodes across all seven stages — which reads like success
until you look at the distribution:

| stage | nodes | concepts per node | verdict |
|---|---|---|---|
| pre-A1 | 7 | 2–10 | usable |
| A1 | 4 | 1–9 | thin |
| A2 | 5 | **1, 1, 1, 2, 42** | broken |
| B1 | 5 | 4 each | placeholder |
| B2 | 4 | 4 each | placeholder |
| C1 | 4 | 4 each | placeholder |
| C2 | 4 | 4 each | placeholder |

`SPINE-SAY-WHAT-I-DO` declared 42 concepts when this was written and declares
**35** today, after `HL23`'s two re-staging slices. `SPINE-TALK-ABOUT-PAST` declares
**one**, and stands for the entire past tense of Spanish. Both cannot be one rung
of the same ladder, and HL09 §1 already named this as the mechanism behind the
false A2 claim.

Above A2 the spine is a sketch: sixteen nodes at four concepts each, for three
stages that together carry roughly 70% of the language.

### 3.2 The rebuilt shape: three tiers

```
STAGE          pre-A1 · A1 · A2 · B1 · B2 · C1 · C2          (7, fixed, exists)
  └─ STRAND    FUNCTION · GRAMMAR · LEXICON · SOUND ·         (8, new, §2)
               ETYMOLOGY · CULTURE · IDIOM · TEXT
       └─ NODE  one can-do, ≤6 concepts, realized by 1–3 chapters   (~400 target)
            └─ CHAPTER   3–12 lessons, ≤12 new atoms       (~850 target)
                 └─ LESSON  <300s, ≤3 atoms, ≤1 grammar cell  (~4,950 target)
```

**Node sizing rules, enforced:**

1. No node declares more than **6** concepts. (Policy already forbids more than
   `maxNewAtomsPerChapter` = 12; 6 is the design target, 12 the hard ceiling.)
2. Every node names its **strand** and its **stage**.
3. Every node is realized by **1–3 chapters**. A node needing four is two nodes.
4. Node prerequisites form a DAG within *and across* strands — a FUNCTION node may
   require a GRAMMAR node, which is how "I can say what I did yesterday" is stopped
   from arriving before the preterite cells that make it possible.

### 3.3 The split of `SPINE-SAY-WHAT-I-DO`

Worked as the template for every oversized node. 42 concepts becomes nine nodes
across two stages. Seven of the 42 have since left, to the A1
`SPINE-NAME-EVERYDAY-ACTIONS` rather than to the nodes sketched below — see
`HL23` §8.3 and §9.2 — so the node stands at 35 and the target shape here is
still the target:

| new node | stage | strand | concepts | realized by |
|---|---|---|---|---|
| `SPINE-NAME-AN-ACTION` | A1 | FUNCTION | 3 | infinitive as a naming form |
| `SPINE-SAY-I-DO-AR` | A1 | GRAMMAR | 4 | the `-ar` first-person cell, alone |
| `SPINE-SAY-YOU-DO-AR` | A1 | GRAMMAR | 4 | second person, formal and informal |
| `SPINE-SAY-SOMEONE-DOES-AR` | A1 | GRAMMAR | 4 | third person |
| `SPINE-SAY-I-DO-ER` | A1 | GRAMMAR | 4 | the `-er` family opens |
| `SPINE-SAY-I-DO-IR` | A1 | GRAMMAR | 4 | the `-ir` family opens |
| `SPINE-SAY-WE-DO` | A2 | GRAMMAR | 6 | first-person plural, all three families |
| `SPINE-SAY-YOU-ALL-DO` | A2 | GRAMMAR | 6 | *vosotros* and *ustedes*, both marked |
| `SPINE-SAY-THEY-DO` | A2 | GRAMMAR | 6 | third plural, closing the present |

Nine nodes, ~18 chapters, ~110 lessons — to do what the corpus currently claims
in 14. That ratio, roughly **8×**, is the honest cost of gentleness, and it holds
across the whole design.

### 3.4 Node naming, and why it never says "Spanish"

Node ids stay language-neutral, because §4.4's whole argument depends on it.
`SPINE-SAY-I-DO-AR` is borderline — `-ar` is a Spanish conjugation class. The rule:

- A node may name a **structural role** that most languages have some version of
  (`FIRST-CONJUGATION`, `PERFECTIVE-PAST`, `POLITE-SECOND-PERSON`).
- A node may **not** name a Spanish form (`-ar`, `usted`, `vosotros`).
- Where Spanish needs something with no cross-language analogue, it goes in an
  **extension node** (`ES-EXT-*`), which the mechanism already supports — Spanish
  has 68 of them today.

So the table above is authored as `SPINE-SAY-I-DO-CONJ1`, and Spanish's
`curriculum.json` records that its conjugation 1 is `-ar`. Telugu will map
something else onto the same rung, or declare an omission, exactly as it does now.

---

## 4. What is universal and what is Spanish

The owner's requirement — *"build the spine carefully so that future language
pipelines can build off this spine"* — is the constraint that decides how much of
this work is reusable. Answered strand by strand:

| strand | universality | how another track reuses it |
|---|---|---|
| FUNCTION | **fully universal** | Node-for-node. A Tamil learner reaches "I can decline politely" at the same rung. |
| TEXT | **fully universal** | Genre progression is a human constant, not a Spanish one. |
| GRAMMAR | **universal slots, local filling** | The slot `PERFECTIVE-PAST-1SG` exists everywhere; what fills it is per-track. Tracks with no such category declare an omission. |
| SOUND | **universal slots, local filling** | Slots are `VOWEL-INVENTORY`, `STRESS-RULE`, `ORTHOGRAPHIC-DEPTH`. Spanish's are short; English's would not be. |
| ETYMOLOGY | **universal method, local content** | The Root Ledger (§6.2) is machinery any track can run. Spanish's Latin/Arabic layers are Spanish's. |
| LEXICON | **universal fields, local ordering** | Semantic fields (family, food, work) are shared; frequency ordering is per-language. |
| CULTURE | **universal hooks, local content** | Hooks are `GREETING-PHYSICALITY`, `FORMALITY-SYSTEM`, `MEALTIME-STRUCTURE`. Content is entirely local. |
| IDIOM | **universal hooks, local content** | Hooks are `BODY-IDIOM`, `ANIMAL-IDIOM`, `PROVERB`. Content is entirely local. |

### 4.1 The dividend, stated concretely

Authoring Spanish to C2 produces, for free:

- ~400 spine nodes with stages, strands, prerequisites and can-do statements —
  the ladder every other track climbs.
- The grammar **slot inventory** (§5.1), which is a typological checklist: a new
  track answers "do you have a perfective/imperfective distinction?" rather than
  designing a syllabus from nothing.
- The eight strand ladders and their budgets.
- The chapter and lesson templates, the activity contracts, the gates.

What it does not produce is content. Tamil's C2 course is still ~4,000 Tamil
lessons. But it is 4,000 lessons **with a plan**, which is the difference between
this program finishing and not.

### 4.2 The one place Spanish must not be the template

Spanish is *orthographically shallow* (near one-to-one sound-to-letter) and
*Latin-script*. Sixteen of the 22 tracks are neither. The SOUND and SCRIPT ramps
must therefore be designed with a deliberate note that Spanish's brevity there is
an accident of Spanish — HL08's `maxNewGlyphsPerLesson` already exists precisely
because the script burden was invisible while Spanish was the pilot.

### 4.3 Regional variety as a first-class dimension

Spanish forces a question the other tracks will also face (Portuguese pt-PT/pt-BR,
Arabic's dialects, Hindi/Urdu): **which variety is being taught?**

Rather than pick one and bake it in, variety is a **parameter**:

- Every lesson already carries `variety:`. Today every Spanish lesson says
  `general`, which is the unmarked default HL09 §8.1 forbids.
- Each **split point** — where usage genuinely diverges — is authored **once**,
  naming every major form, with exactly one marked `productive` and the rest
  `receptive`.
- Which form is productive is a **track configuration**, not a rewrite. Changing
  the course from Peninsular to Rioplatense changes a config key and the generated
  drills; it does not change 5,000 lessons.

The major split points, all of which are authored explicitly:

| split | forms | first taught |
|---|---|---|
| 2nd person singular | *tú* · *usted* · *vos* | pre-A1, all three, receptively |
| 2nd person plural | *vosotros* · *ustedes* | A2, both, at the same moment |
| *c/z* realization | distinción · seseo | pre-A1 (SOUND strand, lesson 4) |
| *ll/y* | *lleísmo* · *yeísmo* · *žeísmo* | A1 |
| past reference | *canté* · *he cantado* preference | A2 |
| lexical splits | *coche/carro*, *ordenador/computadora*, *zumo/jugo*, … | as each word arrives |

> A course that never mentions *vos* is teaching a Spanish that does not exist
> anywhere in the Southern Cone — roughly 100 million speakers. Today `vos`
> appears **0 times in 188 lessons**.

### 4.4 The contract that keeps the spine reusable

Enforced by gate, not by good intentions:

1. No spine node id contains a Spanish-specific morpheme or form.
2. No spine node's `canDo` text names a Spanish word.
3. Every Spanish-specific teaching point lives in an `ES-EXT-*` extension node.
4. Every node declares its strand, stage, and concept count; the concept count is
   checked against the ≤6 target and ≤12 ceiling.

Violations are a build failure, because this is the one property that cannot be
retrofitted: a spine with Spanish baked into it is a Spanish syllabus, and the
other 21 tracks would have to start over.

---

## 5. The grammar ramp

The owner's phrasing is *"grammar should be very gently introduced."* This is the
section that makes that mean something, and it is the most consequential part of
the design — grammar is where every comprehensive language book fails.

### 5.1 The unit is the cell, not the paradigm

A **cell** is one filled slot in one paradigm: `PRESENT-INDICATIVE-1SG-CONJ1`
(*hablo*) is a cell. `PRESENT-INDICATIVE-CONJ1` (the whole six-form table) is
**not** a teachable unit; it is six.

The Spanish verb system, counted honestly in cells:

| system | persons | tenses/moods | conjugations | cells |
|---|---|---|---|---|
| indicative, simple | 6 | 5 (pres, pret, imperf, fut, cond) | 3 | 90 |
| subjunctive, simple | 6 | 3 (pres, imperf, fut†) | 3 | 54 |
| imperative | 5 | 2 (affirm, neg) | 3 | 30 |
| compound (haber + participle) | 6 | 8 | 1 | 48 |
| non-finite | — | 3 (inf, ger, part) | 3 | 9 |
| **regular total** | | | | **231** |
| irregular and stem-changing overlays | | | | ~400 |

† future subjunctive is receptive-only, C2, legal and proverbial register.

Roughly **630 verb cells**, before a single noun, article, pronoun or preposition.
At ≤1 new cell per lesson, the verb system alone is ~630 lessons — and that number
is not a problem to be optimised away. It is the honest size of the thing, and the
reason this course is 5,000 lessons rather than 500.

### 5.2 The one-cell rule

> **`maxNewGrammarCellsPerLesson: 1`.**

A lesson may present one new cell. It may *practise* any number of already-taught
cells. It may *show* a table only under §5.3.

This is the rule that most changes what the book looks like. The existing Spanish
chapters 14–18 already work this way — chapter 15 introduces "one bounded singular
pattern apiece for *comer*, *vivir*, *tener*, *hacer*, *estar*", chapter 18 opens
the subjunctive through a single asserted-versus-wanted contrast — and those are
the best chapters in the corpus. This rule generalises what already works.

### 5.2a What the one-cell rule counts

> **The budget counts new *forms*, not new *slots*.**

A cell whose written form is identical to one the learner already holds carries
no new information, and splitting it across lessons is padding rather than
gentleness.

The case that forced this: Spanish's `-ir` present singular is *the same three
endings* as `-er` — `-o`, `-es`, `-e`. Three distinct cells by slot; **one**
fact by form. `ES-C07-vivir` therefore declares all three CONJ3 cells in a
single lesson, and says so in the prose: the third family asks the reader to
learn no new form at all.

The inverse still binds. `-ar` → `-er` changes two of the three slots, so those
two are genuinely new and were taught one lesson apiece (HL-C99d).

### 5.3 No table before its cells

> **A paradigm table may appear only after every cell in it has been individually
> taught.** The table is then a *recap*, never an *introduction*.

This is the concrete form of "never info dump," and it inverts the universal
textbook convention, which opens each tense with the full six-form grid. That grid
is the single steepest step in language pedagogy: six new forms, one new concept,
no retrieval, and an implicit claim that the learner will absorb them by staring.

Consequence: the present indicative of *hablar* — the classic "chapter 2" table —
is completed around **lesson 380** of this course. Everything before that has been
saying real things with the cells already held.

**How to tell a recap from a dump, since a gate cannot.** `info-dump.ts` flags
table *shape*, which is the honest thing for it to measure and means its output
is a list of candidates, not a list of defects. The test is upstream of the
table: does a teaching lesson introduce each row separately, and does the lesson
carrying the table introduce **zero** atoms? A Spanish sweep found 23 tables of
seven rows or more and **no genuine dump among them** — the six-person future
and subjunctive grids are terminal checkpoints of chapters that teach one row
per lesson, and the rest are lists (months, numbers, days), not paradigms.

### 5.4 The grammar order for Spanish

The full ordering is a DAG; this is its spine, with the stage each rung completes.

| # | rung | stage | lessons | the gentleness problem it poses |
|---|---|---|---|---|
| 1 | frozen verb forms as vocabulary (*soy*, *estoy*, *hay*, *quiero*) | pre-A1 | ~14 | Must be taught as *words*, with no hint of a paradigm behind them. |
| 2 | noun gender, recognition only | pre-A1 | ~12 | Recognition before production, always. Never "learn the rules for gender." |
| 3 | definite article, then indefinite | pre-A1 → A1 | ~20 | Four forms, four lessons, not one table. |
| 4 | plural of nouns | A1 | ~10 | The `-s`/`-es` split is two lessons, not one rule with exceptions. |
| 5 | adjective agreement | A1 | ~24 | Gender then number then position — three separate arcs. |
| 6 | present indicative, singular, conj 1→2→3 | A1 | ~54 | Nine cells, nine lessons, spread over ~14 chapters. |
| 7 | *ser* vs *estar* | A1 → A2 | ~40 | One use at a time. Never the two-column comparison chart. |
| 8 | negation, and yes/no | pre-A1 (!) | ~8 | Currently chapter 19; the learner is questioned from chapter 6. Moved to the front. |
| 9 | question formation and intonation | pre-A1 → A1 | ~16 | Spanish's inversion-optional syntax is a *gift*; teach intonation first. |
| 10 | present indicative, plural | A2 | ~40 | Where *vosotros*/*ustedes* is authored as a split, not a footnote. |
| 11 | stem-changing verbs | A2 | ~48 | Four patterns (e→ie, o→ue, e→i, u→ue), one at a time, singular before plural. |
| 12 | *-go* verbs and other irregular singulars | A2 | ~30 | Already well done in chapters 12–13; generalised. |
| 13 | direct object pronouns | A2 | ~30 | **The minefield.** One person at a time; placement its own arc. |
| 14 | indirect object pronouns | A2 | ~30 | Separated from direct by ~4 chapters, or the learner conflates them forever. |
| 15 | double object pronouns and *se lo* | A2 → B1 | ~20 | Only after 13 and 14 are each independently secure. |
| 16 | *gustar* and reverse-subject verbs | A2 | ~24 | Deferred until indirect objects exist, which is why 14 precedes it. |
| 17 | reflexive verbs | A2 | ~30 | True reflexive → reciprocal → inherent → change-of-state. Four arcs. |
| 18 | preterite | A2 | ~66 | Regular singular → regular plural → the irregular stems, in frequency order. |
| 19 | imperfect | A2 | ~26 | Only three irregulars — a genuinely easy tense, and a chance to rest. |
| 20 | preterite/imperfect contrast | A2 → B1 | ~40 | **The wall.** One discourse function at a time: background, interruption, habit, completed, change-of-state. |
| 21 | near future (*ir a*) | A2 | ~10 | Deliberately before the simple future — it is easier and buys time. |
| 22 | simple future, conditional | A2 → B1 | ~34 | One weld, twice; already well handled in chapter 17. |
| 23 | perfect tenses (*haber* + participle) | B1 | ~48 | Participle formation is its own arc before any compound tense. |
| 24 | commands | B1 | ~44 | *tú* affirmative → *usted* → negative → plural. Negative *tú* is where the subjunctive quietly arrives. |
| 25 | present subjunctive | B1 | ~96 | **One trigger at a time.** Never "WEIRDO". See §5.6. |
| 26 | *por* / *para* | B1 | ~28 | Fourteen uses, fourteen lessons, distributed. Never the two-column table. |
| 27 | *se*: passive, impersonal, accidental | B1 | ~34 | Four unrelated constructions that share a spelling. Four separate arcs. |
| 28 | relative clauses | B1 | ~30 | *que* → *quien* → *el que* → *cuyo*, with the subjunctive interaction deferred. |
| 29 | imperfect subjunctive | B2 | ~40 | Both form sets (*-ra*/*-se*), marked for register. |
| 30 | conditional sentences | B2 | ~36 | Three types, taught as three unrelated things before ever being compared. |
| 31 | reported speech and tense shift | B2 | ~34 | Realizes `SPINE-REPORT-WHAT-OTHERS-SAID`. |
| 32 | passive voice, and why Spanish avoids it | B2 | ~20 | The interesting lesson is the avoidance. |
| 33 | subjunctive in relative and adverbial clauses | B2 | ~50 | The second subjunctive mountain. |
| 34 | aspectual periphrases | C1 | ~44 | *ir/venir/andar/llevar/seguir* + gerund; *acabar de*, *ponerse a*, *volver a*. |
| 35 | perfect subjunctives, *si* inversions, archaic futures | C1 | ~30 | |
| 36 | discourse structure, cohesion, information packaging | C1 | ~60 | Clitic doubling, fronting, cleft sentences — where word order becomes meaning. |
| 37 | stylistic and literary syntax | C2 | ~50 | Including Golden Age forms needed to read Cervantes. |

**≈1,270 grammar-strand lessons.** Every one of them ≤5 minutes, one new cell.

### 5.4a What the corpus actually holds, measured

Audited 2026-08-11, after HL-C104 found the indefinite article missing from 91
chapters. **11 of the 28 rungs above are entirely absent from Spanish.**

The largest is rung 10, and it is worth stating plainly because it changes what
this book currently is:

> **The course is singular-only.** 44 verb-paradigm atoms are marked SINGULAR.
> Exactly two are marked PLURAL, and both are *adjective* agreement, not verbs.
> No plural verb form appears as a headword anywhere. A reader who finishes all
> 93 chapters holds five tenses and a mood — and cannot say *we speak*.

Every lesson has been honest about this; each gloss says "singular". That
honesty is precisely why it stayed invisible for so long: nothing was ever
claimed that was not delivered, so no gate could fire.

**Absent:** 1 (*hay*), **10 (present plural)**, 13, 14, 15, 20, 23, 24, 26, 27,
28. **Present:** 2–9, 11, 12, 16–19, 21, 22, 25.

Rung 10 comes before the rest, because 13, 14, 15, 20, 23, 24 and 27 all assume
plural forms exist. The shape is already established by HL-C94..C104: one cell
per lesson, `-er`/`-ir` shared where they genuinely share (§5.2a), a review
chapter when the paradigm closes, and a synthesis where the choice becomes
communicative.

### 5.4b The noun famine

Measured 2026-08-11, while trying to author rung 13 (object pronouns) and
discovering there was nothing to build it out of.

> **At chapter 53 the reader holds 75 lexical atoms, and the concrete nouns
> among them are `café`, `día`, `noche`, `tarde` and `mañana`.** Every other
> noun in the course — *pan*, *agua*, *vino*, *hermano*, *padre*, *mano*,
> *cabeza*, *gato*, *perro* — is introduced at **chapter 78 or later**.

At the same point the reader already holds `NOUN-GENDER`, `NOUN-NUMBER`,
`DEFINITE-ARTICLES`, and both indefinite articles. **The course teaches the
entire apparatus for handling nouns and withholds the nouns.**

This is the same failure as the missing indefinite article (HL-C104) and the
missing plural articles, and it has the same cause: the grammar rungs were
authored as a ladder while vocabulary was left to arrive incidentally, as
whatever a grammar lesson happened to need for an example.

Two consequences are now rules, not observations:

1. **A structure that operates on a category needs members of that category
   already taught.** An object pronoun replaces a noun; authoring one before any
   ordinary noun exists is not a gentle ramp, it is a rung with no wood in it.
2. **Vocabulary is a strand with its own ordering obligation**, not a byproduct
   of grammar examples. When a grammar rung is scheduled, the nouns and verbs it
   will need are part of its prerequisites and must be checked the same way a
   grammar atom is.

The remedy applied at chapter 53 is deliberately small and general: one
feminine noun, one masculine noun, and one noun *derived* from a verb the
reader already holds (*comer* → *comida*), so that from that point every
gender-dependent rule has two plain test cases to run against and the reader
has seen that nouns can be built rather than only received.

### 5.5 Polysemy is not vocabulary

`maxNewSensesPerLesson: 1`. Spanish's high-frequency verbs are polysemous monsters
— *quedar* has ~12 distinct senses, *dejar* ~9, *llevar* ~8, *poner* ~10. Teaching
"*quedar*: to remain, to be located, to arrange to meet, to suit, to have left,
to end up…" in one lesson is an info dump wearing a vocabulary costume.

Each sense is its own atom (`ES-LEX-QUEDAR-REMAIN`, `ES-LEX-QUEDAR-ARRANGE`, …),
its own lesson, and its own R1–R4 schedule. The senses of *quedar* are distributed
across roughly 60 chapters between A2 and C1.

### 5.6 Worked example: the subjunctive, ninety-six lessons

The subjunctive is where comprehensive Spanish courses either give up or produce
the notorious mnemonic. Here is the gentle version, as the template for every
hard system.

**What is never done:** present the mnemonic; present the full conjugation; say
"the subjunctive is used for doubt, emotion, desire…"; contrast indicative and
subjunctive before the learner can produce either.

**The arc:**

| block | lessons | content |
|---|---|---|
| A | 6 | The *contrast*, with one verb: *Sé que habla* vs *Quiero que hable*. No mood is named. The learner just hears that asserting and wanting sound different. Already prototyped in chapter 18 — this generalises it. |
| B | 9 | The form, one cell at a time, for `-ar` singular only. Derived from the *yo* form the learner already holds, which is why rung 12 (*-go* verbs) had to come first. |
| C | 9 | Same for `-er`/`-ir` singular. |
| D | 6 | The name arrives. Only now — after 24 lessons of using it — is the word *subjuntivo* introduced, with its etymology (*sub-* + *iungere*, "yoked underneath", the clause that hangs off another). |
| E | 12 | Trigger 1: volition (*querer que*, *esperar que*, *preferir que*). One verb per lesson. |
| F | 8 | Plural cells. |
| G | 12 | Trigger 2: emotion. One expression per lesson. |
| H | 12 | Trigger 3: doubt and denial — with the *negation flips it* discovery as its own lesson, because it is genuinely surprising and deserves the room. |
| I | 10 | Trigger 4: impersonal expressions. |
| J | 6 | The irregular stems (*sea*, *vaya*, *sepa*, *haya*, *dé*, *esté*), one per lesson. |
| K | 6 | Synthesis: chapters where the learner *chooses* the mood in free production. |

Ninety-six lessons, ~8 hours of study, ~15 chapters. A conventional textbook does
this in one chapter and roughly 90 minutes, and its learners do not acquire it.

**The compounding property.** Block D is only possible because of block B; block
E is only possible because the learner already holds *querer* from rung 11; block
H's surprise only lands because block E built the expectation. That is what "each
lesson should lead to future lessons" means operationally, and §8.2 measures it.

### 5.7 One verb per chapter

> **A chapter introduces at most one new verb.**

Each verb carries an origin the learner is owed — *hablar* has *fābula*, *comer*
has *comedere*, *tener* has *tenēre*. A chapter that lands three verbs either
tells three etymologies at once, which is the info dump §7.3 forbids, or it
skips two of them, which quietly demotes the etymology strand to decoration on
whichever word happened to go first.

The rule is not about counting lessons. It is about what a chapter can *owe* a
reader and still pay. One verb, one origin, one set of forms.

**Measured 2026-08-10:** 12 of Spanish's 19 verb-teaching chapters break this —
chapters 47, 48 and 52 each taught **four** verbs. That burn-down is HL-C99;
its first six slices split chapters 47, 53, 62, 21, 30 and 20. **No Spanish
chapter now teaches four verbs**; 6 of 22 remain crowded, none with more than
three.

> **Where a review chapter earns its keep.** In the chapter-53 split, `gustar`
> sits *after* the review rather than beside the three ordinary verbs. The
> review states the shared shape out loud — the sentence is about the one doing
> it — so that the reverse-subject verb has something to break. A review
> chapter is not only consolidation; it is where a contrast gets its setup.

**Correction, same day.** An earlier revision of this section claimed `beber`,
`preguntar` and `tomar` were "taught with no etymology at all." That was wrong.
All **42** Spanish verbs carry one; those three carry it through `roots:` and
`etymology_hook` rather than an `ES-ETYMON-*` atom, and the measurement had
counted only the atom namespace. The Root Ledger (§6.2) deliberately spans both
namespaces for exactly this reason, and a one-namespace census contradicts it.
The real defect is only the crowding, which is what the rule above addresses.

### 5.8 Review chapters and synthesis chapters

A chapter-final practice *lesson* is not the same thing as a chapter that exists
to consolidate. Two chapter kinds are therefore first-class:

- A **review chapter** introduces **zero** new atoms. It is where a paradigm
  table is finally allowed to appear (§5.3), because by then every cell in it is
  owned. It is usually `sight` rather than `voice`: a table cannot be read aloud,
  and marking it otherwise would be a lie in the drivability data.
- A **synthesis chapter** also introduces zero atoms, and exists so the learner
  *communicates an idea* with what they hold — not recites it. Its test is
  whether the learner chooses correctly under a real communicative pressure
  (register, repair, politeness), and it must be **voice-drivable**.

> Language learning is not learning words; it is being able to communicate
> ideas. A curriculum that only ever teaches has no place where that happens.

**Measured 2026-08-10:** before HL-C98, **none of Spanish's 50 chapters
introduced zero atoms** — there was no point in the entire course at which the
learner only consolidated.

**Worked example — the first paradigm (chapters 15–19).** The three singular
`-ar` present cells were one lesson, `hablo, hablas, habla`, with a three-row
table on first exposure and pro-drop alongside it. HL-C98 makes it five
chapters: *hablo* · pro-drop → *hablas* → *habla* → **review** (the table, now
earned) → **synthesis** (the same conversation held warmly and respectfully,
where the only thing that changes is one letter). This is the reference shape
for every paradigm after it.

---

## 6. Etymology as a working system

The owner's requirement is *"deep etymological roots which is useful from page 1."*
The operative word is **useful**. Etymology in this course is not colour; it is a
vocabulary-acquisition engine, and by B1 it is doing more work than rote learning.

### 6.1 What already exists, and must not be diluted

708 lessons carry an `etymology_hook`; 788 carry a `## The word, taken apart`
section. HL00 calls it "the heart of the lesson… the signature of this
curriculum," and the chapter 1–8 review singled it out as the strongest thing in
the course. HL09 §4.2 already requires one per lesson. This section adds the
machinery that makes it *pay*.

### 6.2 The Root Ledger

Every etymon taught is an atom (`ES-ETYMON-*`, 1,100 references today) and gets a
ledger entry recording **which later lessons cash it in**.

> **`rootLedgerMinReuse: 3`.** A root may be taught only if at least three later
> lessons draw on it. Roots that pay off fewer than three times are cut, or their
> introduction is moved later to where the payoff lives.

This turns the etymology from a sequence of pleasant asides into a **compounding
asset**. Concretely: *tripālium* → *trabajar* is charming on its own. In the
ledger it is registered, and later cashed by *trabajo*, *trabajador*,
*trabajoso*, and — across the cousin layer — French *travail* and English
*travel*. Five payoffs, so it earns its place.

The ledger is also a **generator**: given the ledger and the lesson positions, the
tooling can emit the list of roots whose payoff windows are open, which is what
lets a 5,000-lesson course stay coherent without a human holding it in their head.

### 6.3 Usefulness outranks etymology

**Owner directive, 2026-08-10, and it governs more than the first word:**

> The book has to be useful from page 1. When deciding between etymology and
> usefulness, always choose usefulness.

The etymology is this curriculum's signature. It is not its purpose. A reader
opens page 1 to be able to *say* something, not to admire a root.

So the rule is a division of labour, not a ranking of interest:

- **Etymology never decides which word is taught, or when.** It decides how the
  word already chosen gets explained. §9's selection order is the authority on
  *which* — function first, frequency second, cognate leverage only third.
- **`rootLedgerMinReuse` culls roots, never headwords.** A root that pays off
  fewer than three times is cut or moved. The *word* stays if the word is useful.
- **A word with no honest etymology is still taught if it is the useful one.**
  Say the origin is unsettled, briefly, and move on.

#### The first word, and a decision reversed

An earlier draft of this spec opened the course on `gracias`, on the strength of
HL09 §4.2 — "a word whose etymology is a dead end should not be the first word
taught" — because *grātia* yields *grace*, *gratitude*, *gratis* and
*congratulate*, demonstrating the method in lesson one.

**That was wrong, and it is reversed here. The course opens on `hola`.**

Three reasons, in order of weight:

1. **A greeting is what actually happens first.** Teaching thanks before hello
   inverts the shape of a human encounter. That every course on earth opens with
   *hello* is not blind convention.
2. **`hola` is the gentler first step.** Its difficulty is a silent letter.
   `gracias` opens on the *seseo*/*distinción* split — GRA-thias against
   GRA-sias — so it would confront the learner with a regional fork in lesson 1.
3. **The dead end is load-bearing.** `ES-C01-hola` does not merely admit that the
   origin is unsettled. It refuses the *hello* resemblance as a false friend and
   says why: *"a fake one is worse than none — it teaches a connection your brain
   then has to unlearn."* For a course whose whole method is family resemblance,
   opening by refusing a fake one establishes that the method has integrity,
   before the learner meets seven hundred real ones.

The lesson also carries the silent `h` — a rule true of *every* Spanish `h` the
learner will ever meet — two pure vowels, the register note on when *not* to say
*hola*, and inverted punctuation. HL09 characterised it by one section; read
whole, it is a strong opening.

`gracias` becomes lesson 2. *grātia* arrives one lesson later, which costs
nothing.

#### The general lesson, which is not about Spanish

The reversed decision was already forbidden by this spec's own §9: **function
first, frequency second, cognate leverage third.** `hola` wins on function, and
both words are top-frequency; promoting cognate leverage above both broke the
ordering. The rule existed and was not applied.

Two failures worth naming, because they will recur across 4,950 lessons:

- **Inheriting a prior judgement instead of reading the artifact.** HL09's verdict
  on `ES-C01-hola` was taken at face value; the file was not opened. A summary of
  a lesson is not the lesson.
- **Optimising the opening to demonstrate the method.** That is author-centric —
  it serves the book's argument about itself rather than the reader's first five
  minutes.

### 6.4 The four layers of Spanish, and when each opens

Spanish's history is unusually legible, and each layer is a *system* the learner
can use predictively.

| layer | share | opens | what it gives the learner |
|---|---|---|---|
| **Latin (inherited)** | ~70% | lesson 1 | The core. Sound laws (§6.5) make it predictive. |
| **Arabic** | ~8%, ~1,000 words | A2 | An entire pattern: initial *al-* is a fused article. *alcalde*, *almohada*, *azúcar*, *aceite*, *ojalá*, *álgebra*, *alcohol*. One recognition rule unlocks hundreds of words and 800 years of history. |
| **Amerindian** | small, high-frequency | B1 | Nahuatl (*chocolate*, *tomate*, *aguacate*, *chile*), Quechua (*papa*, *cóndor*, *pampa*), Taíno (*canoa*, *huracán*, *hamaca*, *maíz*), Guaraní (*jaguar*, *tapioca*). Carries the CULTURE strand's contact history. |
| **Learned Latin / Greek** | large, register-marked | B1 → C2 | The formal register, and the entire scientific/academic vocabulary. Where doublets (§6.6) live. |

### 6.5 Sound laws as the master key (C1)

This is the deepest etymological payload, and the point at which the strand stops
teaching words and starts teaching a *decoder*. Latin → Spanish is regular enough
to be learned as rules:

| law | Latin | Spanish | the learner can now predict |
|---|---|---|---|
| F- → h- | *facere*, *ferrum*, *filium* | *hacer*, *hierro*, *hijo* | *fabulāre* → *hablar*; and why *fábula* also exists |
| -CT- → -ch- | *lactem*, *noctem*, *factum* | *leche*, *noche*, *hecho* | *octō* → *ocho*, *strictum* → *estrecho* |
| PL-/CL-/FL- → ll- | *pluvia*, *clāvem*, *flamma* | *lluvia*, *llave*, *llama* | *plēnum* → *lleno* |
| -LY-/-C'L- → -j- | *fīlium*, *oculum* | *hijo*, *ojo* | *apiculam* → *abeja* |
| Ĕ → ie, Ŏ → ue | *petram*, *portam* | *piedra*, *puerta* | why stem-changing verbs change (rung 11 finally *explained*) |

The last row is the moment the course earns its architecture: a grammatical
irregularity the learner has been drilling since A2 turns out to be a phonological
regularity, and the two strands — GRAMMAR and ETYMOLOGY — meet. That connection is
only available to a course that ran both strands the whole way up.

### 6.6 Doublets, and productive morphology

**Doublets** (B1+): the same Latin word entering Spanish twice — once inherited
and worn down, once borrowed intact by scholars.

*cathedra* → *cadera* (hip) and *cátedra* (professorial chair). *delicātum* →
*delgado* (thin) and *delicado* (delicate). *operam* → *obra* and *ópera*.
*integrum* → *entero* and *íntegro*. Each doublet teaches two words, a sound law,
and a register distinction in one lesson.

**Productive morphology** (B1+) is the engine that makes C2 vocabulary arithmetic
survivable. Roughly 40 affixes, each one lesson, each multiplying reach:

| affix | meaning | reach |
|---|---|---|
| *-ción* | action/result noun ← verb | ~2,000 words, and the English *-tion* bridge |
| *-dad* / *-tad* | quality noun ← adjective | ~1,200 |
| *-mente* | adverb ← feminine adjective | unbounded |
| *-ísimo* | absolute superlative | unbounded |
| *-ito* / *-illo* / *-ón* / *-azo* | diminutive/augmentative, with strong regional and affective load | unbounded |
| *des-*, *re-*, *in-*, *pre-*, *sub-*, *anti-* | negation, repetition, position | ~3,000 |
| *-ero*, *-ista*, *-dor* | agent nouns | ~1,500 |
| *-oso*, *-able*, *-ivo* | adjective formation | ~2,000 |

### 6.7 Friends — the words the learner already knows

*"Introduce things like friends from other languages so the brain can make
connections."* — owner, 2026-08-10.

This is the single largest untapped memory aid in a Spanish course for English
speakers, and it follows directly from the etymology the course already teaches.
English borrowed enormously from Latin and French, so a large share of Spanish
vocabulary has an English relative already sitting in the learner's head. A word
introduced with its friend is not a new word; it is a **recognition**.

The rule that keeps this honest, and consistent with the owner's standing
directive that English is the only requirement for the book:

> **English friends are core. Every other language's friends are a bonus layer** —
> visually skippable, never gating comprehension, and never charged against the
> script or vocabulary ramp.

#### The five kinds of friend

**1. Plain friends** — the connection is visible with no help.
*familia/family · nación/nation · importante/important · hospital/hospital ·
animal/animal · idea/idea*. Free vocabulary, and the reason a beginner is never
starting from zero.

**2. Systematic friends** — the connection is a *rule*, so it transfers to
thousands of words the course never teaches individually. These are the highest-
value lessons in the whole ETYMOLOGY strand, and they interlock with the
productive morphology of §6.6:

| Spanish | English | reach | example |
|---|---|---|---|
| *-ción* | *-tion* | ~2,000 | *nación · educación · información* |
| *-dad* / *-tad* | *-ty* | ~1,200 | *ciudad · libertad · universidad* |
| *-mente* | *-ly* | unbounded | *rápidamente · claramente* |
| *-oso* | *-ous* | ~800 | *famoso · nervioso · precioso* |
| *-ario* | *-ary* | ~400 | *necesario · ordinario* |
| *-ismo* / *-ista* | *-ism* / *-ist* | ~900 | *turismo · artista* |
| *es-* + consonant | *s-* + consonant | ~500 | *escuela/school · español/Spanish · estudiante/student · especial/special* |
| *-ncia* | *-nce* | ~400 | *importancia · diferencia* |

One lesson on *-ción* hands the learner more words than fifty vocabulary lessons.
This is the mechanism behind §6.8's derived reach, seen from the learner's side.

**3. Hidden friends** — the connection is real but invisible until the sound laws
of §6.5 reveal it. This is the deepest and most satisfying kind, and it is the
moment etymology stops being history and becomes a **decoder**:

| Spanish | hidden English friend | the bridge |
|---|---|---|
| *leche* | *lactose*, *lactic* | *lactem*, -CT- → -ch- |
| *noche* | *nocturnal* | *noctem*, same law |
| *hecho* | *fact* | *factum*, same law |
| *hijo* | *filial* | *fīlium*, F- → h- |
| *hierro* | *ferrous* | *ferrum*, F- → h- |
| *lluvia* | *pluvial* | *pluvia*, PL- → ll- |
| *llave* | *clavicle*, *clef* | *clāvem*, CL- → ll- |
| *ojo* | *ocular* | *oculum*, -C'L- → -j- |
| *trabajar* | *travel* | *tripālium*, both descended from the same instrument |

A learner who meets *leche* alongside *lactose* has not memorised a word. They
have understood a sound law, and they will decode *lechería* and *lácteo*
unaided.

**4. False friends** — taught as a formal block in the IDIOM strand (§7.2),
one per lesson, because each is a specific trap: *embarazada* (not embarrassed),
*éxito* (not exit), *sensible* (sensitive), *actualmente* (currently),
*realizar* (to carry out), *molestar* (to bother), *asistir* (to attend),
*constipado* (having a cold).

**5. Cousins beyond English** — the bonus layer. French, Italian, Portuguese and
Catalan reflexes of the same etymon, plus the non-Romance layers the course
already teaches: the Arabic layer connects *azúcar/sugar*, *alcohol*, *álgebra*,
*café*; Nahuatl gives *chocolate*, *tomate*, *aguacate/avocado*; Greek supplies
the entire scientific register.

This is where the multi-track repository pays off, and where HL-C48 becomes
valuable rather than decorative: the cousin panel can be **generated** rather
than hand-typed. A reader who knows French sees *hijo · fils · figlio · filho*;
a reader who does not sees nothing missing.

**Generate it from `roots:`, not from `concept_tag`** — this paragraph said
`concept_tag` until HL-C88 measured what that join actually returns, and the two
keys mean different things. A cousin panel claims *reflexes of the same etymon*.
`concept_tag` joins lessons that teach the **same idea**, which is not the same
claim and is frequently not true of the words it pairs:

| join | `VERB-GO` returns | is it a cousin set? |
|---|---|---|
| `concept_tag` | *ir · andare · aller · eō, īre* | **No.** Spanish *ir* is from *īre*, Italian *andare* from *ambitāre*, French *aller* from a third source entirely. Three unrelated verbs, presented as relatives. |
| `roots:` (`hora-latin`) | *la hora · heure · ora · hora* | **Yes.** All four are reflexes of *hōra*. |

Building the panel on `concept_tag` would therefore emit **false etymology at
scale**, in the one layer of the course whose whole value is that its etymology
can be trusted. Use `roots:`.

Measured reach at the time of writing: **76 Spanish lessons** carry a `roots:`
slug shared with at least one other Romance track. (`concept_tag` would have
offered 63, so the correct key costs nothing in coverage.)

**But `roots:` is not yet sufficient either, and this is the open blocker.** The
field records every etymon a lesson *discusses*, not the etymon of its
**headword**, and a cousin panel needs the latter. `IT-C20-incontrare` declares
`cognoscere-latin` because a second meeting-verb hides in its past tense — true
of the lesson, false as a claim that *incontrare* descends from *cognoscere* —
and that pairs it with Spanish *conocer*. Auditing the strict candidate set by
hand found three such pairs out of 25.

Before the panel can be generated, the schema needs to answer the question the
panel asks: either a `headword_root:` field, or a convention that the first
entry in `roots:` is the headword's own. A one-root heuristic gets 20 of 21
pairs right, which is a proxy, not a fix.

#### The budget and the guardrail

- Friends are an **aid**, never a teaching claim. A friend never counts toward the
  lesson's ≤3 atoms, because it is not new material — it is a hook onto material
  the learner already has.
- **One friend per lesson**, so the aid does not become the info dump it exists to
  prevent.
- A friend must be **real**. The etymology has to be defensible, and where a
  resemblance is coincidence it is taught as a false friend or not at all — the
  *hola*/*hello* non-relationship (§6.3) is the model.

### 6.8 The consequence for the arithmetic

HL09 §3 sizes C2 at ~16,000 words and therefore ~8,000 lessons at 2 atoms each.
That assumed every word is taught individually. With §6.6, it is not:

> **Taught atoms** and **derived reach** are different quantities, and the course
> measures both.

A learner who holds *trabajar* and the *-dor*/*-oso*/*-ción* rules holds
*trabajador* and *trabajoso* without a lesson each. The gate is honest about this:
derived reach is only claimed for affixes actually taught, applied to stems
actually taught, and is reported **separately** from taught atoms — never merged
into a single flattering number.

This is what brings the course in at ~4,950 lessons rather than ~8,000, and it is
a genuine pedagogical improvement rather than a shortcut: teaching *-ción* once is
better than teaching 2,000 nouns that all end in it.

---

## 7. Culture, idiom, register, and the info-dump gate

### 7.1 The CULTURE strand

Today the Spanish track's 188 lessons contain 12 `ES-CULTURE-*` references — to
**exactly one distinct culture atom**. Culture is not currently a strand; it is
an occasional aside.

The strand's rule: **`maxNewCultureClaimsPerLesson: 2`**, and every chapter carries
at least one culture or region note (already HL09 §8's chapter contract — this
gives it content).

The ladder:

| stage | culture content |
|---|---|
| pre-A1 | Where Spanish is spoken, and how many ways. Greeting physicality (*besos*, handshake, *abrazo*) and its regional variation. The three formality systems. Two surnames. |
| A1 | Mealtimes and their hours. *Sobremesa*. The *siesta* as myth vs practice. Naming days and months against the *santoral*. Regional lexical splits as they arrive. |
| A2 | *Fiestas* and the ritual year. Music by region. Food geography. Family structure and address terms. The *usted*/*vos* map, now productively. |
| B1 | The history blocks: Roman Hispania · al-Andalus and 711–1492 · the *conquista* and language contact · independence · the 20th century · Spanish in the United States. Each is 6–12 lessons, and each is tied to the ETYMOLOGY layer it explains. |
| B2 | Literature entry — Lorca, Neruda, Borges, Machado in short excerpt. Cinema. The *Boom*. Regional press and how it differs. |
| C1 | The politics of language: the RAE and its authority, inclusive-language debate, *Spanglish*, indigenous-language policy, language and class. |
| C2 | Cultural weight: what *pueblo*, *patria*, *mestizaje*, *madre*, *dignidad* carry that no dictionary gloss holds. Realizes `SPINE-READ-CULTURAL-WEIGHT`. |

### 7.2 The IDIOM strand, and its admission rule

Idioms are the hardest thing to ramp gently, because they are by definition not
compositional — their meaning is not derivable from parts the learner holds. So
the strand has the strictest admission rule in the course:

> **An idiom may be taught only when every word in it is already taught** — or it
> is taught as an unanalysed **formula** at rung S0/S1, and its analysis is
> scheduled as a specific later lesson, recorded at the point of introduction.

The second clause is what allows *de nada* at lesson 8 and its analysis (*rēs
nāta*, "no thing born") at A2 without a forward reference. The deferral is data,
not a promise — the gate checks the scheduled analysis lesson exists.

`maxNewIdiomsPerLesson: 1`. The ladder:

| stage | idiom content |
|---|---|
| pre-A1 | Transparent formulas only: *de nada*, *por favor*, *hasta luego*, *lo siento*. |
| A1 | The *tener* idioms — *tener hambre/sed/frío/sueño/prisa/razón/años*. The best possible idiom entry point because they are **systematic**: one pattern, ten payoffs, and English's *be* vs Spanish's *have* is a genuine contrast worth a lesson. |
| A2 | *Hacer* weather idioms. *Dar* idioms. Discourse markers: *bueno*, *pues*, *o sea*, *venga*, *vale* — with their heavy regional load. |
| B1 | Body idioms (*costar un ojo de la cara*, *no tener pelos en la lengua*). Animal idioms. Colour idioms. False friends as a formal block (*embarazada*, *éxito*, *sensible*, *actualmente*, *molestar*, *realizar*) — each one its own lesson, because each one is a specific trap. |
| B2 | *Refranes*: 40–60 proverbs, each with its cultural reading and its cousin in other Romance languages. Verb + preposition collocations. Irony and humour marking. |
| C1 | Regional idiom contrast — the same idea in Mexico, Argentina, Spain, Colombia. Register-marked idiom, including the vulgar layer taught **receptively and labelled**. |
| C2 | Literary and archaic idiom; idiom in poetry and song; the idioms that carry class and political signal. |

### 7.3 The info-dump gate

The owner's rule — *"will not info dump ever"* — becomes a measurement.

> **`maxRuleStatementsPerLesson: 1`.** A lesson may state one rule.

A **rule statement** is any sentence of the form "X is used for…", "X always /
never…", "there are N kinds of X", or any table introducing more than one new
row of material. Detection is heuristic and report-only first, per the HL05
precedent, but the failure patterns it targets are specific and known:

| the pattern | where it usually appears | what this course does instead |
|---|---|---|
| the six-form conjugation table on first contact | every textbook, chapter 2 | §5.3 — cells first, table as recap |
| *por* vs *para* as a two-column list of 14 uses | every textbook, ~chapter 12 | 28 lessons, one use each, spread across B1 |
| "WEIRDO" / any subjunctive mnemonic | every textbook | §5.6 — 90 lessons, one trigger at a time |
| ser/estar as a comparison chart | every textbook | one use at a time, ~40 lessons |
| the full pronoun grid | every textbook | direct and indirect separated by four chapters |
| a vocabulary list of 20 items under a topic heading | every textbook | ≤3 atoms per lesson, no exceptions |
| "irregular verbs" as an appendix | every textbook | in frequency order, in the body, one per lesson |

The gate's real value is as a **review aid**: a lesson that trips it is not
automatically wrong, but it is automatically read by a human before merge.

---

### 7.4 Writing for someone who knows nothing

*"Please make this very dummy friendly."* — owner, 2026-08-10.

Gentleness is not only step size; it is also **how the page talks**. A course can
have a perfect ramp and still lose a beginner in its first paragraph by assuming
they know what a *verb* is.

The prose rules, enforced at review:

- **No unexplained metalanguage.** See §7.5 — the words for talking about
  language are themselves taught, on their own ramp.
- **One idea per paragraph. Short sentences.** If a sentence needs a comma to
  hold two clauses together, it is usually two sentences.
- **Analogy before definition.** *"The `-o` on the end is Spanish's way of
  saying 'I'm the one doing it' — it does the job English does with the separate
  word 'I'."* is better than a definition of a first-person morpheme.
- **Show, then name.** The learner uses the thing for many lessons before it gets
  a technical name. The subjunctive arc (§5.6) waits 24 lessons before saying
  *subjuntivo*; that is the pattern everywhere.
- **Never these words:** *simply*, *just*, *obviously*, *of course*, *as you
  know*, *it should be clear that*. Every one of them tells a struggling reader
  that the fault is theirs. This is a lint rule, not a style preference.
- **Never apologise for the language.** No "unfortunately Spanish has three
  conjugations." Difficulty framed as hostile is difficulty the reader braces
  against.
- **Answer the question the beginner actually has.** Usually *"why is it like
  that?"* — which is precisely what the ETYMOLOGY strand exists to answer, and
  the reason this course can be gentle without being shallow.

### 7.5 The metalanguage ramp

The hidden prerequisite of every language textbook: it assumes the reader knows
grammar *vocabulary*. A book that says "the first-person singular present
indicative of a regular `-ar` verb" has used six technical terms to describe one
form, and a beginner who has never studied grammar understands none of them.

So **metalanguage is a taught strand of its own**, inside GRAMMAR, at the same
gentleness as everything else: **one new term per lesson, introduced only when the
learner has already met the thing it names.**

| stage | terms introduced | introduced only after |
|---|---|---|
| pre-A1 | *word*, *sound*, *letter*, *phrase* | — |
| pre-A1 | *noun*, *name-word* | the learner has used a dozen nouns |
| A1 | *verb*, *action word* | the learner has used *soy*, *estoy*, *hablo* |
| A1 | *gender*, *masculine*, *feminine* | ~20 lessons of hearing *el*/*la* |
| A1 | *ending*, *stem* | the learner has seen *habl-o* / *habl-as* |
| A1 | *singular*, *plural* | plurals have been used |
| A1 | *subject*, *person* | *yo*/*tú*/*él* are secure |
| A2 | *conjugation*, *tense*, *regular*, *irregular* | one full singular row exists |
| A2 | *object*, *direct*, *indirect* | the pronoun arcs have begun |
| A2 | *past*, *preterite*, *imperfect*, *aspect* | both pasts are in use |
| B1 | *mood*, *indicative*, *subjunctive* | §5.6 block D, 24 lessons in |
| B1 | *clause*, *subordinate*, *relative* | relative clauses are being used |
| B2 | *voice*, *passive*, *register*, *reported speech* | each is in use |
| C1 | *aspect*, *periphrasis*, *cohesion*, *implicature* | each is in use |

Roughly 40 terms across the whole course, at one per lesson, all of them named
**after** the learner can already do the thing. A term is also re-glossed on
reappearance after a long gap, because a reader at chapter 400 should never have
to search backward for what *preterite* meant.

This is the difference between a book a beginner can read alone and one that
needs a teacher standing next to it.

## 8. The lesson contract, sharpened

HL09 §4 gives the contract. This adds three things it does not have.

### 8.1 The five-minute shape

Derived from `ES-C17-comer-futuro`, which is the corpus's best-shaped lesson, not
invented:

| section | seconds | function |
|---|---|---|
| Warm-up | ~30 | Retrieval of the **previous 1–3 lessons' atoms**. This is the R1 window, closed by construction (HL09 §7.2's option (b)). |
| Teach | ~90 | The one new thing. One cell, or ≤3 atoms, or one sense. |
| The word, taken apart | ~45 | The etymology. Ledger-registered (§6.2). |
| Guided practice | ~90 | 3–4 `hl-activity` items with compiled answer sets. |
| Wrap-up recall | ~30 | Retrieval of what was just taught, plus one forward hook. |
| **total** | **≤285** | leaving headroom under the 300-second gate |

The Warm-up slot is doing structural work: it is where R1 closure lives, so the
reinforcement schedule is a *consequence of the lesson template* rather than an
extra obligation an author might forget.

### 8.2 No dead ends — the compounding requirement

The owner's *"each lesson should lead to future lessons"* is made measurable in
two directions:

- **Downstream reach ≥ 1** (`minDownstreamReach`). Every atom a lesson introduces
  must appear in some later lesson's `requires.knowledge` or `practises.knowledge`.
  An atom with zero downstream reach is a dead end and is reported. Today
  **474 atoms corpus-wide are never revisited**, so this gate has real work to do.
- **Upstream debt = 0.** Every atom a lesson *uses* must have been introduced
  earlier. This is HL09 §4.1's forward-reference rule; **424 forward references**
  exist corpus-wide today.

Together these two say: the course is a connected graph with no orphans and no
loans. That is the formal statement of "gentle."

A third, softer measure — the **payoff chain**: each chapter's payoff lesson
should draw on at least one atom from each of the three preceding chapters, so
the reader feels the course accumulating rather than resetting.

### 8.3 Drivability, by construction

HL08 already derives `coreModality` ∈ {voice, sight, pen} per lesson and per
block, computes the drivable prefix, exports narration, and (HL-C15) prints a
modality sign beside every chapter in the book. The marking the owner asks for
**already exists**. What this spec adds is a design obligation:

> **Spanish is authored voice-first.** Target: ≥95% of the track `voice`, and
> **100% of chapters drivable from their first lesson** (no chapter opens on a
> sight lesson).

Corpus-wide today: 66% drivable, 138 chapters unstartable by ear. Spanish has no
script burden, so there is no excuse for it to be below 95%. Where a lesson
genuinely needs eyes — an accentuation rule, a doublet comparison — the visual
part goes in a **detachable segment** (HL08's block modality), so the lesson stays
drivable and the segment is picked up later at a desk.

Everything a voice assistant needs is already in the data: the narration export
(`narration/*.json`), `[PAUSE Ns]` directives, `[YOU SAY: …]` cues, and the
`hl-activity` contract with `prompt`, `answer`, `accepted`, `feedback`, and
`response_seconds`.

---

## 9. The chapter contract

HL09 §8 gives five requirements. Two additions, both consequences of §2:

6. **Strand declaration.** A chapter names the strands it advances (FUNCTION plus
   ≥2), and the ten-chapter window rule (§2.3) is checked across the track.
7. **Variety marking.** No chapter is `general`. Every chapter declares which
   Spanish it teaches, and any split point it touches is authored per §4.3.

Chapter size: **3–12 lessons**, ≤12 new atoms. The current Spanish maximum is 15
lessons in one chapter, which HL09 §7.2 showed makes R1 closure impossible for its
opening material.

---

## 10. The companion app

The owner's requirement is *"a companion app that allows you to constantly
practice."* Language Ladder exists and has Learn mode, per-language frontier
progression, and mixed review. Four additions make it a practice engine for a
5,000-lesson course.

### 10.1 Mastery is per atom, not per lesson

Today progression is per lesson: complete it, move on. At 5,000 lessons and
~10,000 atoms this is not enough — the unit the learner forgets is the **atom**.

```ts
interface AtomMastery {
  atom: string;              // ES-LEX-GRACIAS
  introducedAt: number;      // sequence position of the teaching lesson
  strength: number;          // 0..1, decaying
  lastSeen: timestamp;
  dueAt: timestamp;
  lapses: number;
}
```

The corpus's R1–R4 windows guarantee the *material* to practise every atom exists.
The app's scheduler decides **when this learner** sees it, from their own record.
Corpus schedule and learner schedule are different things, and conflating them is
why `reviews_of` never reinforced anything.

### 10.2 Voice mode is the primary mode

Not an accessibility feature — the primary way this course is consumed, per the
drivable-course design. The loop: narration TTS → prompt → learner speaks → ASR →
score against the `hl-activity` accepted set → spoken feedback → next.

Requirements the corpus already satisfies: every activity carries `accepted`
variants and `response_seconds`; every lesson carries pause and speech cues; every
sight/pen segment is separately marked so voice mode can skip it and queue it.

### 10.3 Synthesis drills

HL09 §6 requires one synthesis activity per chapter — a prompt whose correct
answer is an utterance the course has never shown. The app generates more of them
on demand from the learner's own held atoms and rungs:

> *"You are hungry, it is morning, and you are speaking to your boss. Say what
> you are going to eat."*

The answer is assembled from *por la mañana* + *voy a comer* + a food word +
*usted*-register — four things held, never combined. The generator's constraint is
exactly the forward-reference rule (§8.2) run in reverse: only atoms with
`strength > threshold`, only rungs at or below current.

### 10.4 The book and the app are the same data

Unchanged from the existing contract, and worth restating because it is what makes
a 10,000-page book maintainable: `core/*.json` + `spanish/lessons/*.md` are the
source; the book is generated; the app reads the same ASTs; hashes gate the drift.
All 41 Spanish chapters already generate this way (HL-C77).

---

## 11. The arithmetic

Revised from HL09 §3, which assumed all vocabulary is taught atom by atom. §6.8
shows it is not.

| stage | taught atoms (cum.) | derived reach (cum.) | lessons | chapters | cum. lessons |
|---|---|---|---|---|---|
| pre-A1 | 300 | 300 | 180 | 30 | 180 |
| A1 | 700 | 750 | 230 | 38 | 410 |
| A2 | 1,400 | 1,600 | 380 | 90 | 790 |
| B1 | 2,700 | 3,400 | 700 | 117 | 1,490 |
| B2 | 4,300 | 6,000 | 850 | 142 | 2,340 |
| C1 | 6,800 | 11,000 | 1,300 | 217 | 3,640 |
| C2 | 9,300 | 16,000+ | 1,300 | 217 | **4,940** |

Chapter numbering runs continuously: pre-A1 1–30, A1 31–68, A2 69–158,
B1 159–275, B2 276–417, C1 418–634, C2 635–851.

**≈4,950 lessons · ≈851 chapters · ≈400 spine nodes.**

At ~2 pages per lesson plus chapter front matter, that is **≈10,500 pages** —
which lands exactly where the owner said it may.

### 11.1 One curriculum, many books — but only one book now

The canonical artifact is **the curriculum**, not any book. Books are **derived
views** over it, and the pipeline already works this way: `core/*.json` plus
`spanish/lessons/*.md` are the source, and all 41 Spanish chapters generate from
the same ASTs Language Ladder consumes (HL-C77).

**Right now, exactly one book is built** (owner decision, 2026-08-10): one
continuous volume, the whole curriculum, no splitting. The single arc is the
deliverable — a learner reads forward from *gracias* to Cervantes without ever
changing artifact, and the strands only read as strands if they are visibly
continuous.

Other editions are **deliberately deferred**, not designed away. Because the
curriculum is the source, each is later a generation target rather than a
rewrite:

| future edition | what it filters | status |
|---|---|---|
| driving edition | voice-capable lessons only; sight and pen blocks omitted | HL-C43, already in the backlog |
| per-part editions | one stage each, for readers who want a smaller file | deferred |
| script/writing companion | pen lessons and ductus only | deferred |
| reference edition | the recap tables, the paradigms, the root index | deferred |

Nothing in the authoring is allowed to assume a particular edition. That is what
keeps this a curriculum with books rather than a book with data behind it.

### 11.2 What one 10,500-page book costs the pipeline

Recorded so it is budgeted rather than discovered mid-build:

- **Scale.** The Spanish book is 214 pages today. 10,500 is ~50×. The `latexmk`
  loop, the LaTeX warning gate, and the per-chapter hash checks must stay linear
  in chapter count; the all-books CI job currently builds 22 books in one pass
  and will need this one budgeted separately.
- **Bookmarks and cross-references.** ~851 chapters and ~4,950 lesson sections is
  a large `hyperref` destination table. The duplicate-destination gate that
  HL-B33 and its siblings drove to zero must hold at fifty times the size.
- **Front and back matter.** A book this size needs a real apparatus: the root
  index, the atom index, the grammar-cell index, and an English-first glossary.
  HL-C50 already built the back-matter machinery for the current 22 books.

Study time at one lesson per day: 13.5 years. At ten lessons per day: 16 months.
The course is designed for the second, and the app's scheduler assumes sessions,
not single lessons.

**Honesty (HL09 §10):** vocabulary targets are **editorial** planning figures, not
claims about any awarding body's syllabus. Derived reach is reported separately
from taught atoms and never merged. No level is claimed until HL09 §3.1 is
satisfied — every node realized, vocabulary met, zero over-budget lessons, and
every atom revisited at least twice.

---

## 12. The stage map

The course, stage by stage. Each block is a run of chapters; lesson counts are
design targets.

> **Chapter counts are floors, not budgets.** Owner directive, 2026-08-10:
> *"Don't constrain yourself by the number of chapters in the book. Keep writing
> as many chapters as you need for a very smooth gentle ramp."* Where the
> one-cell rule (§5.2), the one-verb rule (§5.7) or a needed review/synthesis
> chapter (§5.8) pushes a block past its number here, the block grows and the
> numbering shifts. Lesson **ids** are stable slugs and never renumber with it.

### 12.1 pre-A1 — 30 chapters, ~180 lessons

Everything is a formula. No paradigm exists yet. Sentence rungs S0–S2.

| block | ch | lessons | content |
|---|---|---|---|
| 1 | 1–3 | 18 | *gracias* (§6.3), *de nada*, *por favor*, *hola*, *adiós*. The five Spanish vowels — pure, unreduced, and a genuine advantage over English. First Root Ledger entries. |
| 2 | 4–6 | 18 | **Yes and no.** *sí*, *no*, *vale*/*bueno*, negation by putting *no* before the verb. Moved from chapter 19 to here, because the learner is questioned from chapter 6 and today cannot decline. |
| 3 | 7–9 | 18 | Names: *me llamo*, *¿cómo te llamas?*, *soy*, *mucho gusto*. *yo* and *tú* — both currently absent from the whole path. |
| 4 | 10–12 | 18 | The three formality systems, receptively: *tú* · *usted* · *vos*. Two surnames. Greeting physicality by region. |
| 5 | 13–15 | 18 | Wellbeing: *¿cómo estás?*, *estoy bien/mal/regular*, *¿y tú?*. *estoy* as a word, not a paradigm — and the learner can finally answer the question chapter 13 asks. |
| 6 | 16–18 | 18 | Time-of-day greetings; *buenos días/tardes/noches*; why *días* is plural. |
| 7 | 19–21 | 18 | Leave-taking: *hasta luego/mañana/pronto*, *nos vemos*, *chao*. Regional preference marked. |
| 8 | 22–24 | 18 | Numbers 0–10. *hay*. Asking *¿cuánto?* |
| 9 | 25–27 | 18 | Repair: *perdón*, *lo siento*, *¿cómo?*, *más despacio, por favor*, *no entiendo*. The survival kit, and the first genuinely useful conversation. |
| 10 | 28–30 | 18 | Gender, **recognition only**: *el*/*la* as things you hear, not a rule to apply. Consolidation and the stage's synthesis chapters. |

**Sound strand:** the 5 vowels · stress and the written accent · *ñ* · *rr* ·
*c/z* (distinción vs seseo, marked from the start) · *j*/*g* · silent *h*.
Spanish's orthographic shallowness means SOUND is essentially *finished* by A1 —
which is why this course can afford its grammar ramp.

### 12.2 A1 — 38 chapters, ~230 lessons

The first paradigm opens, one cell at a time. Rungs S2–S4.

| block | ch | lessons | content |
|---|---|---|---|
| 11 | 31–34 | 24 | Articles: *el*, *la*, *los*, *las* — four lessons, then indefinite. Noun plurals, the `-s`/`-es` split as two lessons. |
| 12 | 35–39 | 30 | Present indicative, **conj 1 singular**: *hablo* · *hablas* · *habla*. Three cells, ~9 lessons, plus the infinitive as a naming form. |
| 13 | 40–43 | 24 | Adjective agreement: gender, then number, then position. Colours arrive here as the vehicle. |
| 14 | 44–48 | 30 | Present indicative, **conj 2 and 3 singular**: *como*, *vivo*. Six more cells. |
| 15 | 49–52 | 24 | *ser* vs *estar*, one use at a time: identity · origin · profession · location · condition. Never the comparison chart. |
| 16 | 53–56 | 24 | Question words, one per lesson: *qué*, *quién*, *dónde*, *cuándo*, *cómo*, *cuánto*, *por qué*. Intonation before inversion. |
| 17 | 57–60 | 24 | Lexicon: family · house · food · the body. First `-ito` diminutive. |
| 18 | 61–64 | 24 | Numbers 11–100 · days · months · dates. The *santoral* and why days are named as they are. |
| 19 | 65–68 | 26 | The *tener* idioms as a system (§7.2). Possessives. Consolidation. |

### 12.3 A2 — 90 chapters, ~380 lessons

Plurals, the past, and the pronoun minefield. Rungs S4–S7.

| block | ch | content |
|---|---|---|
| 20 | 69–76 | Present plural, all three conjugations. *vosotros* **and** *ustedes*, authored as a split, taught together, both productive-capable. |
| 21 | 77–84 | Stem-changers: e→ie, o→ue, e→i, u→ue. One pattern per two chapters, singular before plural. |
| 22 | 85–90 | *-go* verbs and irregular singulars. (Generalises the existing chapters 12–13, which already do this well.) |
| 23 | 91–96 | Direct object pronouns, one person at a time; placement as its own arc. |
| 24 | 97–102 | Indirect object pronouns — deliberately four chapters after direct. Then *gustar* and the reverse-subject verbs, which only work once indirect objects exist. |
| 25 | 103–110 | Reflexives: true reflexive → reciprocal → inherent → change-of-state. Daily routine as the vehicle. |
| 26 | 111–122 | **The preterite.** Regular singular → regular plural → irregular stems in frequency order (*fui*, *hice*, *dije*, *tuve*, *estuve*, *pude*, *puse*, *supe*, *quise*, *vine*, *traje*). Eleven chapters. |
| 27 | 123–126 | **The imperfect.** Only three irregulars — a deliberate rest after block 26. |
| 28 | 127–133 | The preterite/imperfect contrast, one discourse function at a time: background · interruption · habit · completed event · change of state. Storytelling begins. |
| 29 | 134–137 | Near future (*ir a*), then simple future and conditional. |
| 30 | 138–144 | **The Arabic layer** (§6.4): *al-* as a fused article, 8% of the lexicon, and 781–1492 as the CULTURE block that explains it. |
| 31 | 145–152 | Comparatives, superlatives, *-ísimo*. Productive suffixes open: *-ción*, *-dad*, *-mente*. Lexicon: work · travel · health · shopping. |
| 32 | 153–158 | *Refranes* block 1. Discourse markers. Fiestas and the ritual year. Consolidation and stage synthesis. |

### 12.4 B1 — 117 chapters, ~700 lessons

The subjunctive, and the point where etymology becomes productive. Rungs S7–S9.

| block | ch | content |
|---|---|---|
| 33 | 159–170 | Participles, then the perfect tenses. *he hablado* → *había hablado* → *habré*/*habría hablado*. |
| 34 | 171–181 | Commands: *tú* affirmative → *usted* → negative → plural. Negative *tú* is where the subjunctive quietly arrives, before it is named. |
| 35 | 182–196 | **The subjunctive, ninety-six lessons** (§5.6). Fifteen chapters. |
| 36 | 197–203 | *por* / *para*, fourteen uses, fourteen lessons, distributed across the block rather than tabled. |
| 37 | 204–211 | *se*: passive · impersonal · accidental · reflexive. Four separate arcs for four unrelated constructions. |
| 38 | 212–218 | Relative clauses: *que* → *quien* → *el que* → *cuyo*. Subjunctive interaction deferred to B2. |
| 39 | 219–232 | **The history blocks:** Roman Hispania · al-Andalus · the *conquista* and language contact · independence · the 20th century · Spanish in the US. Each tied to the etymological layer it explains — this is where the Amerindian layer opens. |
| 40 | 233–248 | Lexicon expansion: education · technology · emotion · opinion · environment · media. Doublets (§6.6) as a formal block. |
| 41 | 249–262 | Idiom: body · animal · colour. **False friends**, one per lesson. |
| 42 | 263–275 | TEXT strand: narrative · blog · news brief · personal letter. Extended storytelling with the full past system. |

### 12.5 B2 — 142 chapters, ~850 lessons

Argument, nuance, and the second subjunctive mountain. Rungs S9–S11.

| block | ch | content |
|---|---|---|
| 43 | 276–288 | Imperfect subjunctive, both form sets (*-ra*/*-se*), register-marked. |
| 44 | 289–300 | Conditional sentences — three types, taught as three unrelated things long before they are ever compared. |
| 45 | 301–312 | Reported speech and the tense shift. |
| 46 | 313–320 | Passive voice, and the more interesting lesson: why Spanish avoids it. |
| 47 | 321–337 | Subjunctive in relative and adverbial clauses. The second mountain. |
| 48 | 338–355 | The polysemous verbs (§5.5): *quedar* · *dejar* · *llevar* · *poner* · *dar* · *echar*. One sense per lesson. |
| 49 | 356–375 | Lexicon: politics · economy · science · law · art. Productive morphology extended: *-ismo*, *-ista*, *-ez*, *-ura*, *-aje*. |
| 50 | 376–395 | Register shifting. Formal writing. The argumentative essay, the opinion column, the report. |
| 51 | 396–410 | **Literature entry:** Lorca, Machado, Neruda, Borges in short excerpt, each with its cultural and etymological reading. |
| 52 | 411–417 | *Refranes* block 2, with Romance cousins. Irony and humour marking. Stage synthesis. |

### 12.6 C1 — 217 chapters, ~1,300 lessons

Where the strands converge. Rungs S11–S12.

| block | ch | content |
|---|---|---|
| 53 | 418–440 | Aspectual periphrases: *ir/venir/andar/llevar/seguir* + gerund; *acabar de*, *ponerse a*, *volver a*, *dejar de*. |
| 54 | 441–455 | Perfect subjunctives, *si*-clause inversions, the future subjunctive receptively. |
| 55 | 456–485 | Discourse structure: clitic doubling · fronting · clefts · information packaging. Where Spanish word order becomes meaning. |
| 56 | 486–510 | **Sound laws** (§6.5). The master key: F→h, -CT-→-ch-, PL/CL/FL→ll, Ĕ→ie, Ŏ→ue. Stem-changing verbs, drilled since A2, are finally *explained*. |
| 57 | 511–545 | Productive morphology completed — ~40 affixes, and the point where derived reach overtakes taught atoms. |
| 58 | 546–580 | **Regional variety, receptively and seriously:** Rioplatense · Caribbean · Andean · Mexican · Peninsular · Andalusian · Canarian. Real listening, real transcripts. Realizes `SPINE-FOLLOW-REGIONAL-VARIATION`. |
| 59 | 581–610 | Registers: legal · medical · journalistic · academic · *coloquial* · vulgar (receptive, labelled). Realizes `SPINE-SHIFT-REGISTER`. |
| 60 | 611–624 | Implicature, irony, understatement. Realizes `SPINE-INFER-IMPLICIT-MEANING`. |
| 61 | 625–634 | The politics of language: the RAE, inclusive language, *Spanglish*, indigenous-language policy. Academic writing and literary analysis. |

### 12.7 C2 — 217 chapters, ~1,300 lessons

Reading the language as a native reader does. Rung S12 throughout.

| block | content |
|---|---|
| 62 | Stylistics: connotation, register-shading, the choice between near-synonyms. Realizes `SPINE-EXPRESS-FINE-SHADES`. |
| 63 | **Older Spanish:** Golden Age orthography and morphology · *vuestra merced* → *usted* · *Cervantes* read in the original · Quevedo, Góngora · the medieval *jarchas* and the *Cantar de Mio Cid*. Realizes `SPINE-READ-LITERARY-AND-CLASSICAL`. |
| 64 | Latin American literary voices in depth: Rulfo, García Márquez, Cortázar, Bolaño, Castellanos, Poniatowska. |
| 65 | Dialectology proper — isoglosses, *voseo* maps, the *yeísmo* front, Caribbean *s*-aspiration. |
| 66 | Translation and paraphrase. Summarising several sources into one argument. Realizes `SPINE-SUMMARIZE-FROM-SOURCES`. |
| 67 | **Comparative Romance** — the same Latin etymon reflected in Spanish, Portuguese, Italian, French, Catalan and Romanian. This is where the multi-track spine pays off spectacularly, and where the cousin layer stops being a bonus and becomes the point. |
| 68 | Humour, wordplay, poetry, song. Cultural weight: *pueblo*, *patria*, *mestizaje*, *dignidad*. Realizes `SPINE-READ-CULTURAL-WEIGHT`. |

---

## 13. What happens to the existing 188 lessons

**Nothing is discarded.** The etymological work in the existing corpus is the
strongest thing in it and took a long time to build.

| disposition | count (est.) | what happens |
|---|---|---|
| absorbed unchanged | ~110 | Re-sequenced into the new spine; `practises.knowledge` wired for R1/R2. |
| split | ~45 → ~95 | Lessons carrying more than one grammar cell or more than one sense are divided per §5.2/§5.5. |
| re-homed | ~25 | Moved to a different stage — chapter 19's *sí*/*no* moves to pre-A1 block 2; chapters 26 and 31's vocabulary moves earlier so chapters 7–8 stop reaching forward for it. |
| rewritten | ~8 | Where the lesson states a rule the new order contradicts. |

Net: 188 → ~240 lessons, occupying ~5% of the finished course, mostly in pre-A1
and A1. The existing 41 chapters become roughly the first 68.

Two things must be fixed during absorption because they are learner-visible holes
already named in HL09 §4.1: the learner cannot say *no*, and cannot say *I am*.

---

## 14. Delivery

Small tranches, per repo standard. The unblockers come first because everything
downstream is unverifiable without them.

| # | tranche | signal |
|---|---|---|
| 0 | **Unblockers** — HL09 steps 1–3: measure order integrity, reinforcement windows, forward references; finish the schema-v2 migration; wire `practises.knowledge` to close R1/R2 | Gap report publishes all three per track; zero sequence-less Spanish lessons |
| 1 | Add the strand dimension to `spine.json`; add the §2.2 budgets to `chapter-policy.json`, report-only | Every existing node carries a strand; new gates run and report |
| 2 | Split every oversized node (§3.3), starting with `SPINE-SAY-WHAT-I-DO` | No node declares >12 concepts; target ≤6 measured |
| 3 | Author the grammar **cell inventory** for Spanish (§5.1) as data | ~630 verb cells enumerated with prerequisites; the DAG validates |
| 4 | Build the Root Ledger (§6.2) over the 1,100 existing etymon references | Every root reports its payoff count; roots below 3 are listed |
| 5 | Enforce the info-dump gate (§7.3), report-only | Every lesson reports rule-statement count; the corpus's worst offenders are listed |
| 6 | Absorb the existing 188 lessons into the new spine (§13) | Zero forward references and zero dead-end atoms in Spanish |
| 7 | Author pre-A1 to its full 180 lessons | HL09 §3.1 satisfied at pre-A1; ≥95% drivable; every chapter has intro, thread, culture note |
| 8 | Per-atom mastery and voice mode in the app (§10.1, §10.2) | The app schedules from atom strength; a full chapter is completable hands-free |
| 9 | A1, then A2, then B1… | §3.1 satisfied per stage before the next begins |
| 10 | Lift the spine, strands and gates into the other 21 tracks | §4.4's contract holds; a second track climbs the same ladder |

**Nothing may claim a level before tranche 7**, and each stage gate is HL09 §3.1
in full.

---

## 15. Acceptance criteria

- [ ] `spine.json` carries a strand per node; no node exceeds 12 concepts, target ≤6.
- [ ] No spine node id or `canDo` names a Spanish-specific form (§4.4).
- [ ] `maxNewGrammarCellsPerLesson: 1` is measured and reported.
- [ ] No paradigm table appears before all its cells are taught (§5.3).
- [ ] Every lesson has an etymology hook, and every root has ≥3 ledger payoffs.
- [ ] Friends are English-anchored and never counted as taught atoms; cousin panels are generated from `roots:`, not hand-typed, and not from `concept_tag` (§6.7).
- [ ] No lesson uses an unglossed grammar term before the metalanguage ramp introduces it (§7.5).
- [ ] The banned-word lint (*simply*, *just*, *obviously*, *as you know*) passes (§7.4).
- [ ] Exactly one book is generated, and no lesson assumes which edition it appears in (§11.1).
- [ ] Every chapter advances FUNCTION + ≥2 strands; every strand advances in every ten-chapter window.
- [ ] Every chapter declares a variety; zero Spanish lessons remain `variety: general`.
- [ ] Zero forward references and zero dead-end atoms in Spanish.
- [ ] ≥95% of Spanish lessons are `voice`; every chapter is drivable from its first lesson.
- [ ] Taught atoms and derived reach are reported separately, never merged.
- [ ] Level claims are gated on HL09 §3.1, and reported as *in progress at X* until then.

---

## 16. Owner decisions, recorded

Both were open when this spec was drafted and were settled on 2026-08-10. They
are recorded here rather than in a session, because the repository is the source
of truth.

### 16.1 The productive variety is neutral Latin American

*ustedes*, seseo, *tú*. Peninsular and Rioplatense are **fully receptive from
pre-A1** — the learner meets *vosotros*, distinción and *vos* from block 4 and is
never surprised by them, but produces the neutral American forms.

Rationale: the largest speaker base, and `vosotros` is the form a learner can most
safely hold by recognition alone. Per §4.3 this is a **track configuration key**,
not a property of the lessons: every split point is authored with all forms and
one marked `productive`, so a Peninsular edition is a config change and a drill
regeneration, never a rewrite. That property is the reusable part — Portuguese
(pt-PT/pt-BR), Arabic and Hindi/Urdu all need the same mechanism.

### 16.2 One curriculum; for now, exactly one book

Not seven volumes, and not a split of any kind yet. The curriculum is canonical
and books are derived views over it; today the build emits **one continuous
book** containing the whole course. The driving edition, per-part editions, a
writing companion and a reference edition are all deferred generation targets,
listed in §11.1 so they are visibly postponed rather than forgotten.

The consequence for authoring: no lesson may assume which edition it is being
read in.

### 16.3 Still genuinely open

Nothing blocking. Two questions that only become real at B1 and can be answered
from evidence by then rather than guessed now:

1. **How much C1/C2 listening needs recorded audio** rather than TTS. The regional
   variety block (§12.6, block 58) is the first place synthetic speech may stop
   being adequate, since the whole point is hearing how varieties actually differ.
2. **Whether the derived-reach model needs its own gate.** §6.8 reports derived
   reach separately from taught atoms; if it ever starts being cited as coverage,
   it needs the same fail-closed treatment level claims got in HL09 §3.1.

---

## Appendix A — Chapter 1, written out

Every rule in this spec is a constraint on lessons nobody has written yet, which
makes them easy to agree with and hard to check. So here is the first chapter in
full, in the canonical format, as the reference implementation of the design.

**This appendix is a sample, not corpus.** It is not loaded by `loadLessons`, it
generates no book chapter, and it does not replace the published Spanish Chapter
1. Moving it into `spanish/lessons/` is HL-C85's job and is an editorial
decision, not a mechanical one.

### What it is demonstrating

| rule | where to look |
|---|---|
| Usefulness outranks etymology, so a greeting opens (§6.3) | Lesson 1 |
| One friend per lesson, English-anchored (§6.7) | every lesson's *taken apart* |
| An honest dead end, stated briefly and put to work (§6.3) | Lesson 1 |
| ≤3 new atoms (§2.2) | `introduces.knowledge` |
| **Zero** grammar cells at pre-A1 (§5.4 rung 1) | no lesson declares `teaches_cells` |
| No metalanguage before §7.5 introduces it | no lesson says *verb*, *noun*, *phrase* |
| Warm-up closes R1 by construction (§8.1) | every lesson after the first |
| Voice-first, chapter drivable from lesson 1 (§8.3) | no table, no sight cue, anywhere |
| Variety declared, never `general` (§4.3) | frontmatter `variety:` |
| Culture is a strand, not decoration (§7.1) | Lessons 1 and 5 |
| The payoff uses only what was taught (§9) | Lesson 6 |

### The chapter's shape

Six lessons, ~28 minutes total. Nine new atoms against a chapter budget of
twelve. The learner leaves able to conduct a complete, polite, real exchange —
not to recite five words.

---

#### Lesson 1 — `ES-C01-hola`

```yaml
schema_version: 2
id: ES-C01-hola
spine_node: SPINE-MEET-GREET
strand: FUNCTION
sequence: 10
chapter: 1
type: vocabulary
headword: hola
gloss: hello
prerequisites: []
sounds: [silent-h, vowel-o, vowel-a]
roots: []
etymology_hook: "hola's origin is genuinely unsettled, and the resemblance to English hello is a false friend"
duration:
  max_seconds: 240
requires:
  knowledge: []
introduces:
  knowledge: [ES-LEX-HOLA, ES-SOUND-H-SILENT, ES-REGISTER-HOLA-INFORMAL]
practises:
  knowledge: [ES-LEX-HOLA, ES-SOUND-H-SILENT]
skills: [listening, speaking]
modes: [interpersonal]
strands: [meaning-input, meaning-output]
register: informal
variety: american-neutral
```

**Teach.** Your first Spanish word is **hola**. It means *hello*.

Say it **OH-la**. The **h** makes no sound at all.

That is not a quirk of this one word. Every **h** in Spanish is silent — written,
never spoken — and you will meet hundreds of them without ever hearing one.

The two vowels are pure: **OH**, **AH**. They do not slide the way English vowels
do. Hold each one steady and stop.

**The word, taken apart.** Here the honest answer is that nobody knows.

*Hola* is an old hailing call, and that is as far as the evidence firmly goes.
You will see it confidently traced to English *hello*, or to Arabic *wallāh*, or
to a shout across water. Treat all of those as folk etymology: plausible, not
established.

The resemblance to English **hello** is a **false friend**. English *hello* grew
out of a Germanic hailing root — *hail*, *holler* — a different family entirely.
The two words look alike by coincidence.

> Why say so this firmly, in the first lesson? Because almost everything that
> follows is built on real family resemblance between Spanish and English, and
> a *fake* one is worse than none — it teaches a connection your brain then has
> to unlearn. Enjoy *hola*. Do not file it beside *hello*.

**When not to say it.** *Hola* is informal — the *hi* you would use with a
friend, a child, a shopkeeper you know. It carries no time of day and no
deference, which is exactly why, meeting someone for the first time or in a
formal setting, Spanish reaches past it. Those words are coming.

**Wrap-up.** How do you greet someone? *(Hola.)* What does the **h** sound like?
*(Nothing at all.)* Is *hola* a safe thing to say to your new boss? *(Not
really — something else is coming for that.)*

> **Why this word first.** Usefulness outranks etymology (§6.3). A greeting is
> what actually happens first in an encounter, its only difficulty is a silent
> letter, and the honest refusal of a false friend sets the standard of evidence
> for the seven hundred real ones ahead.

---

#### Lesson 2 — `ES-C01-gracias`

```yaml
id: ES-C01-gracias
spine_node: SPINE-COURTESY-THANK
sequence: 20
type: vocabulary
headword: gracias
gloss: thank you
prerequisites: [ES-C01-hola]
sounds: [vowel-a, stress-penultimate, seseo-c]
roots: [latin-gratia]
etymology_hook: "gracias comes from Latin gratia, the same root inside English grace, gratitude and gratis"
requires:
  knowledge: [ES-LEX-HOLA]
introduces:
  knowledge: [ES-LEX-GRACIAS, ES-ETYMON-GRATIA]
practises:
  knowledge: [ES-LEX-GRACIAS, ES-ETYMON-GRATIA, ES-LEX-HOLA]
variety: american-neutral
```

**Warm-up.** *(retrieval, closes R1 for lesson 1)* Greet someone. *(Hola.)* Say
it once, out loud, and let the **h** stay silent.

**Teach.** **Gracias** means *thank you*.

Say it in two beats: **GRA-sias** across Latin America and southern Spain,
**GRA-thias** in most of Spain. Both are correct, both are heard. You are
learning the first, and you will understand the second.

**The word, taken apart.** *Gracias* comes from Latin **grātia** — kindness
freely given, a favour nobody owed you.

You already own a piece of that word. English took **grātia** too, and kept it in
**grace**, in **gratitude**, in **gratis** (given for free), and in
**congratulate**. When you say *gracias*, you are saying a word English also
kept, just worn down differently by eight hundred years of Spanish mouths.

That is not a coincidence to memorise. It is the first of many — and, unlike
*hola* and *hello*, it is real.

**Guided practice.** Say **gracias** three times. Say it to someone who hands you
something. Say it to a stranger who holds a door.

**Wrap-up.** What is *thank you*? *(Gracias.)* Which English word hides the same
root? *(Grace — or gratitude, or gratis.)*

#### Lesson 3 — `ES-C01-de-nada`

```yaml
id: ES-C01-de-nada
spine_node: SPINE-COURTESY-THANK
sequence: 30
type: vocabulary
headword: de nada
gloss: you're welcome
prerequisites: [ES-C01-gracias]
roots: [latin-res-nata]
etymology_hook: "de nada is literally 'of nothing' — the same modesty English uses in 'not at all'"
requires:
  knowledge: [ES-LEX-GRACIAS]
introduces:
  knowledge: [ES-LEX-DE-NADA]
practises:
  knowledge: [ES-LEX-GRACIAS, ES-LEX-DE-NADA]
variety: american-neutral
```

**Warm-up.** *(retrieval, closes R1 for lesson 1)* How do you say *thank you*?
*(Gracias.)* Say it once, out loud.

**Teach.** When someone thanks you, you answer **de nada**.

**The word, taken apart.** *De nada* means, word for word, *of nothing*. You are
not saying "you are welcome" — you are saying "it was nothing, don't mention it."

English does the same modest shrug with **not at all** and **don't mention it**.
Spanish just picked a different way to say the same small kindness.

*Nada* has a strange and lovely history: it comes from Latin **rēs nāta**, "a
thing born." Over centuries the phrase *no… rēs nāta* — "not a born thing" —
collapsed, the *rēs* fell away, and the word for *thing born* became the word for
**nothing**. A word that once meant *something* now means its opposite.

**Guided practice.** Someone says *gracias*. You answer. Say both halves: **—
Gracias. — De nada.**

**Wrap-up.** What do you answer to *gracias*? *(De nada.)* What does it literally
mean? *(Of nothing.)*

---

#### Lesson 4 — `ES-C01-por-favor`

```yaml
id: ES-C01-por-favor
spine_node: SPINE-POLITE-REQUEST-REPAIR
sequence: 40
headword: por favor
gloss: please
prerequisites: [ES-C01-gracias]
roots: [latin-favor]
etymology_hook: "por favor is 'as a favour' — English kept favor from the same Latin word"
requires:
  knowledge: [ES-LEX-GRACIAS]
introduces:
  knowledge: [ES-LEX-POR-FAVOR]
practises:
  knowledge: [ES-LEX-GRACIAS, ES-LEX-DE-NADA, ES-LEX-POR-FAVOR]
variety: american-neutral
```

**Warm-up.** *Gracias* — and the answer? *(De nada.)*

**Teach.** **Por favor** means *please*. You put it at the end of what you ask
for, and it turns a demand into a request.

**The word, taken apart.** *Favor* is the same word as English **favour**. Both
come straight from Latin **favor**, goodwill. So *por favor* is "as a favour" —
you are asking someone to do you a kindness, which is exactly what *please* does
in English without ever saying so out loud.

**Guided practice.** Say **por favor** on its own. Now say **gracias, por
favor** — and notice it sounds wrong, because it is. *Por favor* asks. *Gracias*
thanks. One comes before you get the thing; the other comes after.

**Wrap-up.** Which word asks, and which thanks? *(Por favor asks. Gracias
thanks.)*

---

#### Lesson 5 — `ES-C01-adios`

```yaml
id: ES-C01-adios
spine_node: SPINE-TAKE-LEAVE
sequence: 50
headword: adiós
gloss: goodbye
prerequisites: [ES-C01-hola]
roots: [latin-ad-deum]
etymology_hook: "adiós is 'to God' — the same farewell inside French adieu and English goodbye"
requires:
  knowledge: [ES-LEX-HOLA]
introduces:
  knowledge: [ES-LEX-ADIOS, ES-CULTURE-FAREWELL-WEIGHT]
practises:
  knowledge: [ES-LEX-ADIOS, ES-LEX-HOLA, ES-LEX-GRACIAS]
variety: american-neutral
```

**Warm-up.** Greet someone. *(Hola.)* Thank them. *(Gracias.)*

**Teach.** **Adiós** is *goodbye*.

**The word, taken apart.** *Adiós* is **a Dios** — "to God." It is the short
remains of an old blessing: *I commend you to God* until we meet again.

The whole of Western Europe said the same thing. French kept **adieu**. Italian
kept **addio**. And English — this is the good part — English says **goodbye**,
which was once **God be with ye**, worn down over centuries until nobody could
hear God in it any more.

So *adiós* and *goodbye* are not related words. They are the *same sentence*,
said in two languages, both eroded past recognition.

**A note on weight.** In much of the Spanish-speaking world *adiós* is a little
final — it can carry a hint of *for a long time*, or *for good*. For an ordinary
"see you later" you will want **hasta luego**, which is coming shortly. Use
*adiós* when you mean it.

**Wrap-up.** How do you say goodbye? *(Adiós.)* What is hiding inside it? *(A
Dios — "to God.")* And inside English *goodbye*? *(God be with ye.)*

---

#### Lesson 6 — `ES-C01-payoff` *(chapter payoff)*

```yaml
id: ES-C01-payoff
spine_node: SPINE-COURTESY-THANK
sequence: 60
type: practice-mix
headword: (none — this lesson introduces nothing)
prerequisites: [ES-C01-hola, ES-C01-gracias, ES-C01-de-nada, ES-C01-por-favor, ES-C01-adios]
requires:
  knowledge: [ES-LEX-HOLA, ES-LEX-GRACIAS, ES-LEX-DE-NADA, ES-LEX-POR-FAVOR, ES-LEX-ADIOS]
introduces:
  knowledge: []
practises:
  knowledge: [ES-LEX-HOLA, ES-LEX-GRACIAS, ES-LEX-DE-NADA, ES-LEX-POR-FAVOR, ES-LEX-ADIOS, ES-ETYMON-GRATIA]
variety: american-neutral
```

**Warm-up.** Five words. Say them: *hola, por favor, gracias, de nada, adiós.*

**The payoff.** You can now run a whole exchange. Not a drill — a real one. You
walk into a shop, you want something, you get it, you leave.

> — **Hola.**
> — Hola.
> — … **por favor.**
> — *(hands it over)*
> — **Gracias.**
> — **De nada.**
> — **Adiós.**

The gap in the middle is the point. You do not yet know the word for the thing
you want, and you do not need it — you can point. Everything *around* the gap,
the entire polite frame a stranger expects, you already have.

**Synthesis.** *(a prompt whose answer the course has never shown)* Someone holds
a door for you and you walk through. Say the right word. *(Gracias.)* They
answer. What do they say? *(De nada.)*

**Wrap-up.** Which word did you learn first? *(Hola — and its **h** is silent.)*
Which word's root you already owned in English? *(Gracias — grace, gratitude,
gratis.)*

### What this chapter deliberately does not do

- **No grammar.** Not one cell. `hola` is not "an interjection", `gracias` is not
  "a plural noun". Those are true and they are §7.5's business, many chapters
  away.
- **No table.** Not a single one, so the chapter is drivable end to end and the
  narration export needs no refusal.
- **No forward reference.** Every word in every example is taught in this chapter.
  The shop exchange has a hole in it *on purpose* rather than borrowing a noun
  from Chapter 26 — which is exactly the reach-sideways failure HL09 §4.1
  diagnosed.
- **No apology for the language.** Nothing is called easy, simple, or obvious.
- **No invented etymology.** Lesson 1 says outright that nobody knows where *hola*
  comes from, and that its likeness to *hello* is a coincidence — and puts that
  refusal to work as the standard of evidence for every real friend that follows.
