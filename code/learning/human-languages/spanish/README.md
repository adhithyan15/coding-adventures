# Spanish

The pilot track for the [Human Languages](../README.md) curriculum. Goal:
absolute-beginner to B1 ("can hold a normal day-to-day conversation") over a
year, delivered two ways from the same underlying content — a **book**
(`book/`, LaTeX, meant for free publication) and **practice units**
(`units/`, ~5-minute pieces consumed during a daily car commute). Framework
details (unit anatomy, spaced-repetition schedule, etymology/gender
methodology) are in
[`HL00`](../../../specs/HL00-human-language-curriculum-framework.md) — this
README is "how to actually use this."

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
7. **An executable five-minute progression.** Chapters 1–3 use HL04 schema v2:
   each lesson has a stable local sequence, a shared-spine node, declared
   knowledge inputs and outputs, typed body blocks, and an independently checked
   duration below 300 seconds.

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

Units are plain Markdown, written in an audio-script style (`[PAUSE]`,
`[REPEAT x2]`, `[YOU SAY: ...]`) so they can be read aloud by you, a
passenger, or (eventually) a voice pipeline — that pipeline doesn't exist
yet; see `HL00`'s "Explicitly Out of Scope" section.

## The book

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

Chapter 1 is generated from the same seven canonical lesson ASTs consumed by
Language Ladder. Run `npm run build && npm run generate:books` from
`code/packages/typescript/human-language-data` after editing those lessons; CI
rejects stale generated TeX or source-hash metadata. Chapters 2–18 are still
handwritten LaTeX during the staged one-source migration. `units/` is legacy
source material, not a second canonical copy.

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
  verbs**) → **tú / usted** (informal vs formal "you") → the *vuestra merced*
  origin of *usted* → cómo → the Latin *qu-* question family → se llama →
  ¿cómo se llama usted? → mucho → mucho gusto → practice.
- **Chapter 4 — How Are You**: gracias → de nada → estar → ¿cómo está usted? →
  regular → practice.
- **Chapter 5 — Farewells**: adiós → hasta → hasta luego / mañana / pronto.
- **Chapter 6 — The First Verbs**: por favor → hablar (the *-ar* template) →
  trabajar → estudiar → hablo español.
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
- **Chapter 15 — Completing the Preterite**: comer/vivir → the strong preterites.
- **Chapter 16 — The Imperfect**: imperfecto → the three irregulars → choosing
  between the two pasts.
- **Chapter 17 — The Future and the Conditional**: futuro → condicional (one
  weld, twice).
- **Chapter 18 — The Subjunctive**: subjuntivo → quiero que (the mood of the
  not-yet-real).

Every noun carries its gender (*el*/*la*) and plural; every pronoun is traced
to its root. **All 18 chapters are authored and in the book (122 pages); Chapter
1 is generated from canonical lessons.**
Lessons are named by **slug** (e.g. `ES-C01-dia`), not numbered — order lives
in the book (which LaTeX auto-numbers) and in `session-map.md`, so inserting a
lesson never renumbers anything.
- **Pronunciation**: [`pronunciation-reference.md`](./pronunciation-reference.md)
  (look-up reference, not a chapter).
- **Later chapters**: skeleton in [`roadmap.md`](./roadmap.md).
- **`units/` (legacy)**: the pre-redesign coarse-grained content (old Part 0
  + Part I). Kept as source material to be re-excavated into deep lessons;
  **superseded**, not the current model. Being migrated `units/` → `lessons/`.

See [`CHANGELOG.md`](./CHANGELOG.md) for the full history including the
redesign.

## Files

- [`lessons/`](./lessons/) — the deep one-word practice lessons (current model).
- [`pronunciation-reference.md`](./pronunciation-reference.md) — sounds, to
  look up on demand.
- [`roadmap.md`](./roadmap.md) — the chapter skeleton.
- [`session-map.md`](./session-map.md) — which lessons make up which session.
- [`book/`](./book/) — the LaTeX book.
- [`units/`](./units/) — **legacy**, pre-redesign; being migrated to `lessons/`.
