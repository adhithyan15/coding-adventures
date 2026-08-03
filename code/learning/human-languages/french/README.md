# French

The second track of the [Human Languages](../README.md) curriculum, built on
the same framework as [Spanish](../spanish/README.md) (see
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md)): one
word per lesson, gone deep; the widest honest web of English cousins; the
cultural/idiomatic *why*; grammar and pronunciation introduced in context,
never front-loaded.

## What's different about the French track

French grounds each word against **English and its closest Romance sibling,
Spanish** — both worn-down Latin. Per `HL00`'s Audience rule, no prior Spanish
is assumed: the text supplies every Spanish form in full, as enrichment, so the
*differences* between the two Latin daughters can become the lesson:

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
- **Chapter 2 — Introducing Yourself**: authored ([`lessons/FR-C02-*`](./lessons/))
  — je, me, (s')appeler, **je m'appelle** ("my name is"), **tu / vous**,
  comment, **comment vous appelez-vous?** ("what's your name?"), enchanté(e),
  practice. In the book. (Every atom — *je*, *me*, *appelle* — traced to its
  root.)
- **Chapter 3 — How Are You**: merci, de rien, aller, *comment ça va*, *comme ci
  comme ça*, practice.
- **Chapter 4 — Farewells**: au revoir, à plus tard, à bientôt, à demain, practice.
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

**All twenty-three chapters are authored and in the book (98 pages).** Chapters
17–23 are generated from the same canonical lesson AST and source hashes that
Language Ladder verifies independently.

## Files

- [`lessons/`](./lessons/) — the deep one-word practice lessons.
- [`pronunciation-reference.md`](./pronunciation-reference.md) — French sounds,
  to look up on demand.
- [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
- [`book/`](./book/) — the LaTeX book (`latexmk -xelatex book.tex`).

Lessons are named by **slug** (e.g. `FR-C01-jour`), never numbered; order
lives in the book (LaTeX auto-numbers) and `session-map.md`.
