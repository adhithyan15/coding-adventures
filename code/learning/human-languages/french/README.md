# French

The second track of the [Human Languages](../README.md) curriculum, built on
the same framework as [Spanish](../spanish/README.md): one
word per lesson, gone deep; the widest honest web of English cousins; the
cultural/idiomatic *why*; grammar and pronunciation introduced in context,
never front-loaded.

## What's different about the French track

French grounds each word against **English and its closest Romance sibling,
Spanish** — both worn-down Latin. No prior Spanish is assumed: the text
supplies every Spanish form in full, as enrichment, so the *differences*
between the two Latin daughters can become the lesson:

- *día* (Spanish) vs. *jour* (French) — same Latin *dies*, but French detoured
  through *diurnum* (→ English *journal*, *journey*).
- *buenos días* (plural, a fossilized blessing) vs. *bonjour* (singular).
- Latin *-ct-* → Spanish *-ch-* (*noche*) vs. French *-it-* (*nuit*).
- Formal "you": Spanish *usted* (← "your grace") vs. French *vous* (the plural
  "you") — same politeness, different mechanism (coming in Chapter 2).

## Exam destination

The track's [assessment contract](assessment-spec.md) targets DILF A1.1 as the
closest official pre-A1 runway, DELF *tout public* at A1–B2, and DALF at C1–C2.
The machine-readable [contract](assessment.json) requires all four skills, the
complete gentle writing ramp, and two timed mocks at every rung. This names the
destination; it does not claim that the current book is exam-ready. French A1
already has a sourced [task-shape inventory](task-shapes/a1.json). The other
inventories, mocks, rubrics, answer keys, calibration, and book-only human
validation remain explicit backlog.

## Progress

- **Chapter 1 — Greetings**: authored ([`lessons/FR-C01-*`](./lessons/)) —
  salut, bien, bon/bonne, le/la/les (gender), jour, bonjour, soir, bonsoir,
  nuit, bonne nuit, practice. In the book.
- **Chapter 2 — Introducing Yourself**: authored
  ([`lessons/FR-C02-*`](./lessons/)) — je, me, (s')appeler, **je m'appelle**
  ("my name is"), **tu / vous**, comment, **comment vous appelez-vous?**
  ("what's your name?"), enchanté(e), practice. In the book. (Every atom —
  *je*, *me*, *appelle* — traced to its root.)
- **Chapter 3 — How Are You**: merci, de rien, aller, *comment ça va*, *comme
  ci comme ça*, practice.
- **Chapter 4 — Farewells**: au revoir, à plus tard, à bientôt, à demain,
  practice.
- **Chapter 5 — The First Verbs**: parler, habiter, travailler, *je parle
  français*, practice.
- **Chapter 6 — Numbers One to Ten**: nombres 1–5, 6–10.
- **Chapter 7 — The Days of the Week**: jours (the planet-gods).
- **Chapter 8 — Telling the Time**: l'heure, midi/minuit.
- **Chapter 9 — Months and Seasons**: mois, saisons.
- **Chapter 10 — Family**: parents, frères/sœurs (Grimm's law).
- **Chapter 11 — Bread, Water, Wine**: pain, eau/vin.
- **Chapter 12 — Numbers Eleven to Twenty**: 11–16, 17–20.
- **Chapter 13 — Colours**: noir/blanc, rouge/bleu.
- **Chapter 14 — To Have, and How Old You Are**: avoir, âge.
- **Chapter 15 — The Compound Past**: passé composé, passé simple.
- **Chapter 16 — To Be, and the Past That Takes It**: être, passé composé with
  *être*.
- **Chapter 17 — Head and Hand**: *la tête*, *la main*.
- **Chapter 18 — Yes and No**: *oui*, *non*.
- **Chapter 19 — Please**: *s'il vous plaît*.
- **Chapter 20 — Sorry**: *je suis désolé(e)*.
- **Chapter 21 — Weather**: *le temps*, *il pleut*.
- **Chapter 22 — Dog and Cat**: *chien*, *chat*.
- **Chapter 23 — Green and Yellow**: *vert*, *jaune*.
- **Chapter 24 — Taking, Asking, Helping, Loving**: *prendre*, *demander*,
  *aider*, *aimer*.
- **Chapter 25 — Verbs of the Mind**: *comprendre*, *penser*, *lire*, *écrire*.
- **Chapter 26 — Hearing, Sleeping, Walking, Running**: *entendre*, *dormir*,
  *marcher*, *courir*.
- **Chapter 27 — Sitting, Standing, Opening, Closing**: *s'asseoir*, *se lever* /
  *debout*, *ouvrir*, *fermer*.
- **Chapter 28 — Coffee, Tea, Milk, Sugar**: *le café*, *le thé*, *le lait*,
  *le sucre*.
- **Chapter 29 — The People You Introduce**: *l'ami(e)*, *la famille*,
  *l'enfant*, *la personne*.
- **Chapter 30 — Cheese, Butter, Salt, Egg**: *le fromage*, *le beurre*,
  *le sel*, *l'œuf*.
- **Chapter 31 — Eyes, Nose, Mouth, Stomach**: *l'œil* (*les yeux*), *le nez*,
  *la bouche*, *le ventre*.

**All thirty-one chapters are authored and in the book (156 pages).**

### The pre-A1 noun tranche (Chapters 28–31)

Sixteen everyday nouns, one per lesson, all filed under the seven pre-A1 spine
nodes so they count toward `levelGate.tracks[french].vocabulary` — the
mechanism confirmed by the Hindi, Arabic and Tamil tranches (HL-C53): the gate
counts distinct `headword:` strings, one per lesson, so sixteen lessons move
the count by exactly sixteen. Measured with `report-cli`:

- pre-A1 distinct headwords: **26 → 42** (shortfall against 300: 274 → 258)
- pre-A1 atoms revisited fewer than twice: **9 → 0** (19 etymology hooks
  waived)
- track-wide distinct headwords: 78 → 94

Every word was checked against the lessons directory before writing: *l'eau*,
*le vin*, *le pain*, *le père/la mère*, *le frère/la sœur*, *la tête* and *la
main* are already taught, so this tranche adds only what was missing —
coffee, tea, milk and sugar (Chapter 28); friend, family, child and person
(Chapter 29); cheese, butter, salt and egg (Chapter 30); eye, nose, mouth and
stomach (Chapter 31), extending Chapter 17's head and hand rather than
repeating it.

French's own signature is foregrounded rather than treated as an
afterthought. Every noun carries its article and its gender is stated at the
point of teaching, never assumed; Chapter 29 lines up four different
relationships between grammatical gender and the person named (*l'ami/l'amie*
changes with its referent, *la famille* is fixed regardless, *l'enfant* is one
spelling with two articles, *la personne* is fixed feminine even naming a
man). The definite-article elision already used once, quietly, at *l'eau*
(Chapter 11) is formalised here for the first time as its own atom,
`FR-GRAMMAR-ELISION-ARTICLE-02`, introduced at *l'ami* and reused at
*l'enfant*, *l'œuf* and *l'œil*. Cognates were checked rather than assumed —
*sel*/*salt*, *œuf*/*egg* and *nez*/*nose* are genuine common-descent cousins
from a shared PIE root, while *lait*/*milk* share no root at all, and
*café*/*coffee* and *beurre*/*butter* are borrowings, not inheritances.

Household objects were considered and dropped, matching the finding all three
prior tranches reported independently: the seven pre-A1 spine nodes are all
social speech acts and hold no concept for naming a concrete object in front
of you.

Etymologies were corrected against sources during authoring rather than
assumed: the popular claim that Roman soldiers were paid in salt (behind
*salary*) appears in no ancient source and is recorded as a later legend
attached to a genuine word (*salārium* really is built on *sal*); the tidy
"*persona* = sound through" story is flagged as doubted on phonetic grounds,
with Etruscan *phersu* given as the likelier root; and *boútūron*
("cow-cheese," behind *beurre*/*butter*) is treated as a probable Greek
folk-reshaping of a foreign word, since butter was never a native Greek or
Roman product.

Reach-back runs at two cadences. Every lesson practises atoms from the one to
three lessons immediately before it, closing HL09's R1 window across each
chapter seam. Each chapter's payoff reaches back further: `FR-C28-sucre`
recovers *si* and the *langue d'oïl* / *langue d'oc* split from Chapter 18;
`FR-C29-personne` closes with a "four kinds of gender" synthesis over its own
chapter and reassesses Chapter 18's *non*; `FR-C30-oeuf` reaches back to
Chapter 29's elision rule; `FR-C31-ventre` is the tranche's own grand payoff,
naming one word from each of the four new chapters and re-closing three
atoms from Chapter 17's *tête* that nothing had revisited since it was
written. Every one of the nine pre-A1 atoms the continuity ledger reported as
revisited fewer than twice before this tranche is now revisited at least
twice, and the tranche adds none of its own below that floor.

All sixteen lessons derive `coreModality: voice` — no table wider than two
columns, and none of the sight-cue phrases the corpus checks for. The book
compiles under XeLaTeX at 156 pages with zero `Missing character` warnings.

### The first verb tranche (Chapters 24–25)

Eight canonical core verbs, one per lesson, taking French from **6 of the 40**
shared core verbs to **14 of 40**. Each of the eight is already taught by
Spanish, Latin and Portuguese under the same concept id, so every one of these
lessons turns a three-way cross-language join into a four-way one.

They are split across two chapters rather than crammed into one because eight
one-verb lessons introduce twenty atoms, well past the twelve a chapter is
allowed (`maxNewAtomsPerChapter`). Split, each chapter lands inside budget —
Chapter 24 introduces eleven atoms, Chapter 25 introduces nine — and each gets
its own capability and its own payoff.

The order is not arbitrary. *Comprendre* **is** *prendre* with *com-* in front,
so *prendre* has to be taught first: Chapter 25 opens by cashing Chapter 24
rather than by teaching a new conjugation. Two more things French does that
English does not are given their own treatment: *aimer* covers **both** "like"
and "love," and adding *bien* to it makes it **weaker** (*je t'aime bien* is
how you turn someone down); *demander* is a false friend and means plain
"ask," never English's forceful "demand."

### The second verb tranche (Chapters 26–27)

Eight more canonical core verbs, one per lesson, taking French from **14 of the
40** shared core verbs to **22 of 40** — the deepest coverage in the corpus.
These eight were realised by **no track anywhere** before this tranche, so the
concept ids `VERB-HEAR`, `VERB-SLEEP`, `VERB-WALK`, `VERB-RUN`, `VERB-SIT`,
`VERB-STAND`, `VERB-OPEN` and `VERB-CLOSE` come off the corpus-wide
"nobody teaches this" list.

Split two-by-two for the same reason as the first tranche: eight one-verb
lessons introduce twenty-two atoms against a chapter budget of twelve. Chapter
26 introduces eleven and Chapter 27 introduces eleven, so both land inside
budget and each carries its own capability and payoff.

Four things this pair of chapters can say that the earlier ones could not:

- ***entendre*** is Latin *intendere*, "**to stretch toward**" — the same verb
  English kept as **intend**. From the oldest French texts it already covers
  hearing, attending, understanding and intending all at once; what happened
  later is that the other senses fell away as *ouïr* (← *audīre*) wore out, and
  they survive only in *entendu*, *s'entendre* "to get along" and *l'entente*.
  English took the *audīre* family instead — *audio*, *audience*, *audible*.
- **the body-position verbs are reflexive**, and ***s'asseoir*** ships with
  **two** accepted paradigms (*je m'assieds* and *je m'assois*). The lesson
  teaches the *-ie-* series whole and marks the *-oi-* singulars as ordinary
  speech whose plurals are much rarer, rather than pretending the two are
  symmetric.
- **French has no single verb for "to stand."** The movement is ***se lever***
  and the state is ***être debout***, which is *de* + *bout*, "**on end**."
- ***ouvrir*** ends in *-ir* and takes ***-er*** endings, the same set as
  *marcher*; ***fermer*** owns them outright, so the two opposites drill as one
  pair.

And two useful everyday facts land here: *ça marche* means "that works, fine,
deal," and *fermer* is Latin *firmāre* — a French door is not shut but **made
firm**, the same verb that gave Italian *fermare* "to stop" and Spanish
*firmar* "to sign."

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capabilities (HL05)

[`chapters.json`](./chapters.json) states what a reader can *do* when they
finish a chapter, and names the lesson that proves it. It is authored intent —
no validator may rewrite it.

**Fifteen of thirty-one chapters are authored: 17–31.** Those are exactly the
chapters whose lessons have been migrated to schema version 2 and so declare
real knowledge atoms. Chapters 1–16 are still schema v1 and carry no
`practises.knowledge`, so a payoff written for them could only assess invented
atoms. They are left out on purpose: an absent entry is debt the gap report can
measure, a stub is a chapter falsely claiming a capability it never delivered.

Representativeness — the share of a chapter's introduced atoms its payoff
actually assesses, floored at 0.5 by `core/chapter-policy.json`:

| Chapter | Payoff lesson | Assessed / introduced |
|---|---|---|
| 17 Head and Hand | `FR-C17-main` | 4 / 8 = 0.50 (on the floor) |
| 18 Yes and No | `FR-C18-non` | 4 / 7 = 0.57 |
| 19 Please | `FR-C19-sil-vous-plait` | 3 / 3 = 1.00 |
| 20 Sorry | `FR-C20-je-suis-desole` | 3 / 3 = 1.00 |
| 21 Weather | `FR-C21-le-temps` | 4 / 4 = 1.00 |
| 22 Dog and Cat | `FR-C22-chien-chat` | 4 / 4 = 1.00 |
| 23 Green and Yellow | `FR-C23-vert-jaune` | 5 / 5 = 1.00 |
| 24 Taking, Asking, Helping, Loving | `FR-C24-aimer` | 7 / 11 = 0.64 |
| 25 Verbs of the Mind | `FR-C25-ecrire` | 6 / 9 = 0.67 |
| 26 Hearing, Sleeping, Walking, Running | `FR-C26-courir` | 9 / 11 = 0.82 |
| 27 Sitting, Standing, Opening, Closing | `FR-C27-fermer` | 8 / 11 = 0.73 |
| 28 Coffee, Tea, Milk, Sugar | `FR-C28-sucre` | 6 / 8 = 0.75 |
| 29 The People You Introduce | `FR-C29-personne` | 6 / 9 = 0.67 |
| 30 Cheese, Butter, Salt, Egg | `FR-C30-oeuf` | 5 / 8 = 0.625 |
| 31 Eyes, Nose, Mouth, Stomach | `FR-C31-ventre` | 5 / 8 = 0.625 |

Chapters 17, 18 and 24–27 have **no terminal consolidation lesson**, so their
payoff is the last lesson by `sequence`. Chapter 18 still clears the floor
because *non* reassesses *oui*; Chapter 17 sits exactly on it. No `assesses`
list is padded — a shortfall is a signal that the chapter needs a real practice
lesson.

Chapters 24 and 25 do something the earlier payoffs do not: they assess atoms
from **earlier chapters** as well as their own, which is HL09 §7's spaced-
retrieval rule (the corpus measured 50% of taught atoms as never revisited at
all). `FR-C24-aimer` re-practises *le chien* and *le chat* from Chapter 22, *le
vert* from Chapter 23 and the *s'il vous plaît* request from Chapter 19;
`FR-C25-ecrire` re-practises *la main* and its *manus* family from Chapter 17 —
which is not decoration, since *manuscript* is *manus* + *scrībere* — plus
*prendre* and *aimer* from Chapter 24. Across the tranche, **eight French atoms
that had never been revisited by anything now are**: `FR-LEX-MAIN-02`,
`FR-ETYMON-MAIN-04`, `FR-LEX-CHIEN-02`, `FR-LEX-CHAT-04`, `FR-LEX-PLEASE-02`,
`FR-GRAMMAR-PLEASE-REGISTER-04`, `FR-LEX-NON-02` and `FR-SOUND-NON-03`.

Chapters 26 and 27 reach back at **two** cadences rather than one. Every lesson
practises atoms from the **one to three lessons immediately before it**, across
the chapter seam — which is the only cadence that can close HL09's R1 window
(n+1 … n+3), because a chapter-*end* payoff is out of range for everything the
chapter opened with. On top of that, each payoff reaches several chapters back:
`FR-C26-courir` uses *courir*'s hard *c* to re-earn why *canis* became *chien*
(Chapter 22), and runs *il fait chaud* / *il pleut* (Chapter 21) and *ne … pas*
(Chapter 18) against the new verbs; `FR-C27-fermer` opens and closes *la main*
(Chapter 17), closes because *il pleut* (Chapter 21), and asks in both registers
before apologising (Chapters 19 and 20).

Measured effect: **eleven more French atoms that nothing had ever revisited now
are** — `FR-ETYMON-CHIEN-03`, `FR-ETYMON-CHAT-05`, `FR-ETYMON-MAINTENIR-05`,
`FR-SOUND-MAIN-03`, `FR-GRAMMAR-IL-FAIT-03`, `FR-LEX-IL-PLEUT-04`,
`FR-GRAMMAR-NEGATION-04`, `FR-LEX-DESOLE-02`, `FR-PRAGMATICS-SORRY-04`,
`FR-ETYMON-COMPRENDRE-02` and `FR-ETYMON-PENSER-05`. French's
`atomsNeverRevisited` falls 33 → 25 while its atom count rises 54 → 76. Two of
the tranche's own new atoms are still unrevisited — `FR-ETYMON-COURIR-11` and
`FR-ETYMON-LEVER-05` — and that is recorded rather than papered over.

## Files

- [`lessons/`](./lessons/) — the deep one-word practice lessons.
- [`chapters.json`](./chapters.json) — the HL05 chapter capability ledger.
- [`pronunciation-reference.md`](./pronunciation-reference.md) — French sounds,
  to look up on demand.
- [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
- [`book/`](./book/) — the LaTeX book (`latexmk -xelatex book.tex`).

Lessons are named by **slug** (e.g. `FR-C01-jour`), never numbered; order
lives in the book (LaTeX auto-numbers) and `session-map.md`.
