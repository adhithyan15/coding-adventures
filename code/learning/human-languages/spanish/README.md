# Spanish

The pilot track for the [Human Languages](../README.md) curriculum. Goal:
absolute-beginner to B1 ("can hold a normal day-to-day conversation") over a
year. It comes two ways from the same content: a **book** you can read straight
through (`book/`, free to download), and **short practice pieces** (`units/`,
about five minutes each, meant for a daily commute).

## What this track does deliberately

1. **One word per lesson, gone deep.** Not "the ten greetings" — *hola* is a
   lesson, *buenos días* is a lesson. A few minutes each. See `lessons/`.
2. **The widest honest web of English cousins.** Every word reaches for all
   the English relatives it can *truthfully* claim (e.g. *quiero* ← *quaerere*
   → query, inquire, require, acquire, conquer, exquisite) — because the more
   live connections a word has, the more places it sticks. False cousins are
   flagged, not smuggled in.
3. **The reason, not the rule.** Why *buenos días*, plural? Because it's the
   fossil of a blessing ("may God give you good days"). Each lesson digs out
   the cultural/idiomatic *why*, the part Spanish 101 skips.
4. **Prefixes and suffixes taught in context.** When a root builds its family
   by prefix (*in-* + *-quiry* → *inquiry*), the construction is shown and
   named — a skill that pays off across the learner's whole English
   vocabulary.
5. **Pronunciation inline, never a gate.** No alphabet chapter to sit through.
   Each lesson names the sounds *its* word needs; the full system lives in
   [`pronunciation-reference.md`](./pronunciation-reference.md) to look up on
   demand.
6. **Grammar from zero, contrasted with English.** Grammar concepts are
   introduced in context on the first word that needs them, explained with no
   assumed terminology.
7. **Short enough to actually do.** Every lesson is one word or one phrase and
   fits in a few minutes, so a day with no time still has room for one.

## How to use this in the car

1. Before you drive, open [`session-map.md`](./session-map.md) and find the
   next session you haven't done yet.
2. Read (or have read to you) the units listed for that session, in order —
   due reviews first, then the new unit, then any morphology/practice-mix
   unit. Each unit is self-paced: pause, speak your answer out loud, then
   continue.
3. That's the core block, ~15-25 minutes. Longer drive? Keep going into the
   bonus queue — extra review units, never anything you haven't earned yet.
4. Next drive, start from the top of the next session. Don't skip ahead —
   the review schedule assumes you did the units in order.

The practice pieces are written to be read out loud — by you, or by a passenger
— so they are full of "say this now" prompts. Pause, answer aloud, then carry
on.

## The book

`book/` is the continuous read: all thirty-five chapters, free under CC BY-SA
4.0. The book prints no audio prompts and no timing cues; the practice pieces
keep them, because that is what they are for.

`book/` is a LaTeX book, compiled with XeLaTeX (`fontspec` + `polyglossia`
— required later for Arabic/Devanagari/Dravidian scripts, so it's the
standard from day one even though Spanish itself doesn't need it).
Licensed CC BY-SA 4.0. To build it locally:

```
cd book && latexmk -xelatex book.tex   # or: ./build.sh / .\build.ps1
```

Requires a LaTeX distribution with `xelatex`/`latexmk` on PATH (MiKTeX or
TeX Live). The compiled PDF isn't committed — it's regenerated from source,
same as build artifacts elsewhere in this repo.

Chapters 1–6, 19–33 and 34–37 are generated from the same canonical lesson ASTs
consumed by Language Ladder. Run `npm run build && npm run generate:books` from
`code/packages/typescript/human-language-data` after editing those lessons; CI
rejects stale generated TeX or source-hash metadata. Chapters 7–18 are still
handwritten LaTeX during the staged one-source migration. `units/` is legacy
source material, not a second canonical copy. The complete PDF builds without
missing glyphs, layout-box warnings, bookmark warnings, duplicate destinations,
LaTeX warnings, or font fallbacks — 262 pages as of Chapters 36–37. (Length is
never a cost here, so the page count is expected to rise with every tranche.)

## Progress

The track was **redesigned** after early feedback: from coarse units (ten
greetings crammed together, a front-loaded pronunciation chapter) to deep
one-word lessons with inline sounds and a full cousin web. Current state:

All built **atom-first** (word first, then assembled) with grammar introduced
exactly where a word needs it:

- **Chapter 1 — Hola and Buenos Días**: hola → bien (+ bueno/buena) → el / la
  → grammatical gender (a separate short support lesson) → día (first noun,
  applies gender; + the plural rule) → buenos días (assembled, introduces
  **agreement**) → practice.
- **Chapter 2 — The Rest of the Greetings**: tarde → buenas tardes
  (**feminine** agreement) → noche → buenas noches → practice.
- **Chapter 3 — Introducing Yourself**: me → llamar → me llamo (**reflexive
  verbs**) → **tú / usted** (the two words) → tú *or* usted (**register**, and
  why *usted* takes he/she forms) → the *vuestra merced* origin of *usted* →
  cómo → ¿cómo? (the **¿** and the question accent) → the Latin *qu-* question
  family → se llama → ¿cómo se llama usted? → mucho → mucho gusto → practice.
- **Chapter 4 — How Are You**: gracias → de nada → estar (the verb and *stāre*)
  → estás / está (**state and location**) → ¿cómo está usted? → regular →
  practice.
- **Chapter 5 — Farewells**: adiós → hasta → hasta luego / mañana / pronto.
- **Chapter 6 — The First Verbs**: por favor → hablar (the *-ar* template) →
  trabajar → estudiar → hablo español.
- **Chapter 7 — The *-er* and *-ir* Verbs**: comer → vivir → beber → qué →
  dónde.

- **Chapter 6 — The First Verbs**: por favor → hablar (the word) → hablo,
  hablas, habla (the ***-ar* template** and pro-drop) → trabajar → estudiar →
  español → hablo español.
- **Chapter 7 — The *-er* and *-ir* Verbs**: comer → vivir → beber → qué → dónde.
- **Chapter 8 — Numbers and Age**: números 1–5, 6–10 → tener → ¿cuántos años?
- **Chapter 9 — *Ser* and *Estar***: ser → ser vs estar → soy de → está en.
- **Chapter 10 — Going, and the Near Future**: ir → ir a + infinitive →
  mi / tu / su.
- **Chapter 11 — Stem-Changing Verbs**: querer → poder → the boot pattern →
  nuestro.
- **Chapter 12 — The *-go* Verb Club**: hacer → decir → the *yo-go* pattern.
- **Chapter 13 — Completing the *-go* Club**: poner → salir → venir.
- **Chapter 14 — The Preterite**: ser/ir preterite → hablar preterite.
- **Chapter 15 — Completing the Preterite**: comer/vivir → the strong
  preterites.
- **Chapter 16 — The Imperfect**: imperfecto → the three irregulars → choosing
  between the two pasts.
- **Chapter 17 — The Future and the Conditional**: futuro → condicional (one
  weld, twice).
- **Chapter 18 — The Subjunctive**: subjuntivo → quiero que (the mood of the
  not-yet-real).
- **Chapters 19–23 — Everyday building blocks**: yes/no → lo siento, then
  perdón → weekdays → negro/blanco (and blanco's Germanic loan) then rojo, then
  azul → parents and siblings, then hermano's silent *h*.
- **Chapters 24–28 — Body, seasons, food, and time**: head and hand → seasons →
  agua, then vino, then pan → months → noon and midnight.
- **Chapters 29–33 — Daily description**: telling time → el tiempo, hace calor,
  llueve → once–quince, dieciséis–diecinueve, Latin's subtractive teens, veinte
  → gato then perro → verde then amarillo.
- **Chapter 34 — Four Verbs of the Mind**: pensar (← *pēnsāre* "to weigh") →
  entender (← *intendere* "to stretch toward") → leer (← *legere* "to gather")
  → escribir (← *scrībere* "to scratch"). The first two are the payoff for the
  e→ie boot Chapter 11 taught.
- **Chapter 35 — Taking, Asking, Helping, and the Backwards Verb**: tomar (no
  settled etymology, and why *coger* is not safe everywhere) → preguntar (←
  *percontārī*, "to sound with a pole"), and why English's one *ask* becomes
  *preguntar* and *pedir* → ayudar (← *adiūtāre*: literally English *aid*) →
  **gustar**, which runs backwards — *me gusta el libro* is "the book pleases
  me", so the verb agrees with the thing, not with you.
- **Chapter 36 — Hearing, Sleeping, Walking, Running**: oír (← *audīre*, and
  *obey* ← *ob-* + *audīre*), with *escuchar* as the deliberate half of
  listening → dormir (← *dormīre*), whose *o→ue* is the boot's other vowel →
  caminar (← *el camino* ← Gaulish *camminus*), beside *andar*, whose origin is
  genuinely disputed → correr (← *currere*), whose family reaches *car* through
  Celtic.
- **Chapter 37 — Opening, Closing, Sitting Down, Standing Up**: abrir (←
  *aperīre*), with the irregular participle *abierto* → cerrar (← *serāre*, the
  **bar across a door** — *not* *claudere*, which English kept and Spanish gave
  up) → sentarse (← \**sedentāre*) → levantarse (← \**levantāre*). Both
  body-position verbs are reflexive and both were built from a Latin present
  participle; standing as a *state* is **estar de pie**, not a verb at all.

Every noun carries its gender (*el*/*la*) and plural; every pronoun is traced
to its root. **All 37 chapters are authored and in the book (262 pages);
Chapters 1–6, 19–33 and 34–37 are generated from canonical lessons.** Lessons are
named by **slug** (e.g. `ES-C01-dia`), not numbered — order lives in the book
(which LaTeX auto-numbers) and in `session-map.md`, so inserting a lesson never
renumbers anything. - **Pronunciation**:
[`pronunciation-reference.md`](./pronunciation-reference.md) (look-up
reference, not a chapter). - **Progression metadata**:
[`roadmap.md`](./roadmap.md) and [`session-map.md`](./session-map.md) still
need reconciliation through Chapter 33; this is tracked as HL-M09 in the shared
backlog. - **`units/` (legacy)**: the pre-redesign coarse-grained content (old
Part 0 + Part I). Kept as source material to be re-excavated into deep lessons;
**superseded**, not the current model. Being migrated `units/` → `lessons/`.

to its root. **All 33 chapters are authored and in the book; Chapters 1–6 and
19–33 are generated from canonical lessons.**
Lessons are named by **slug** (e.g. `ES-C01-dia`), not numbered — order lives
in the book (which LaTeX auto-numbers) and in `session-map.md`, so inserting a
lesson never renumbers anything.
- **Pronunciation**: [`pronunciation-reference.md`](./pronunciation-reference.md)
  (look-up reference, not a chapter).
- **Progression metadata**: [`roadmap.md`](./roadmap.md) and
  [`session-map.md`](./session-map.md) still need reconciliation through
  Chapter 33; this is tracked as HL-M09 in the shared backlog.
- **`units/` (legacy)**: the pre-redesign coarse-grained content (old Part 0
  + Part I). Kept as source material to be re-excavated into deep lessons;
  **superseded**, not the current model. Being migrated `units/` → `lessons/`.

See [`CHANGELOG.md`](./CHANGELOG.md) for the full history including the
redesign.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

### Building the book

```
cd book && latexmk -xelatex book.tex   # or: ./build.sh / .\build.ps1
```

Needs a LaTeX distribution with `xelatex`/`latexmk` on PATH. The PDF is not
committed; it is regenerated from source like any other build artefact.

Chapters 1–6, 19–33 and 34–37 are generated from the same canonical lesson ASTs
consumed by Language Ladder. Run `npm run build && npm run generate:books` from
`code/packages/typescript/human-language-data` after editing those lessons; CI
rejects stale generated TeX or source-hash metadata. Chapters 7–18 are still
handwritten LaTeX during the staged one-source migration. `units/` is legacy
source material, not a second canonical copy.

The book view strips the lessons' delivery cues (`[PAUSE Ns]`, `[YOU SAY: …]`,
`[REPEAT xN]`) and prints book headings in their place; the canonical lesson
files keep every cue, because the spoken view needs them.

## Chapter capabilities

[`chapters.json`](./chapters.json) is the track's HL05 capability ledger. Each
entry says, in the reader's own voice, what finishing that chapter lets them
*do* (`canDo`), and names the lesson that proves it (`payoff`) together with the
knowledge atoms that payoff exercises. It is authored intent, not a derived
cache — no validator may rewrite it.

All 25 Spanish chapters that own a `core/book-generation.json` target are
authored: **1–6**, **19–33** and **34–37**. Chapters **7–18** are deliberately
absent.
Their lessons are still schema v1 with no declared `practises.knowledge`, so
there is no honest payoff to point at; a stub would destroy the very signal the
HL05 gap report exists to measure. That absence is tracked debt, and it clears
when those chapters migrate to schema v2.

Chapters 1–6 end in a terminal `practice-mix` lesson, which is the payoff.
Chapters 19–37 have no practice lesson, so the payoff is the chapter's last
lesson by sequence — the one carrying its recombination and wrap-up recall.

## Files

- [`chapters.json`](./chapters.json) — the HL05 chapter capability ledger.
- [`lessons/`](./lessons/) — the deep one-word practice lessons (current model).
- [`pronunciation-reference.md`](./pronunciation-reference.md) — sounds, to
  look up on demand.
- [`roadmap.md`](./roadmap.md) — the chapter skeleton.
- [`session-map.md`](./session-map.md) — which lessons make up which session.
- [`book/`](./book/) — the LaTeX book.
- [`units/`](./units/) — **legacy**, pre-redesign; being migrated to `lessons/`.
