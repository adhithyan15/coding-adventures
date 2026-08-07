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

**All twenty-five chapters are authored and in the book (114 pages).**

### The verb tranche (Chapters 24–25)

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

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capabilities (HL05)

[`chapters.json`](./chapters.json) states what a reader can *do* when they
finish a chapter, and names the lesson that proves it. It is authored intent —
no validator may rewrite it.

**Nine of twenty-five chapters are authored: 17–25.** Those are exactly the
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

Chapters 17, 18, 24 and 25 have **no terminal consolidation lesson**, so their
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

## Files

- [`lessons/`](./lessons/) — the deep one-word practice lessons.
- [`chapters.json`](./chapters.json) — the HL05 chapter capability ledger.
- [`pronunciation-reference.md`](./pronunciation-reference.md) — French sounds,
  to look up on demand.
- [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
- [`book/`](./book/) — the LaTeX book (`latexmk -xelatex book.tex`).

Lessons are named by **slug** (e.g. `FR-C01-jour`), never numbered; order
lives in the book (LaTeX auto-numbers) and `session-map.md`.
