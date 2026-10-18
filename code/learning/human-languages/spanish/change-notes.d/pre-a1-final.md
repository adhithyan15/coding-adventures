## Unreleased — Pre-A1: the last thirty-five words (chapters 328-334)

### Added

- **Thirty-five pre-A1 vocabulary lessons across seven chapters**, which close
  the pre-A1 vocabulary gate for Spanish. The track entered this tranche at
  **269** distinct headwords taught at or below pre-A1, against the HL09 §3.1
  floor of **300**; it leaves at **304**. Vocabulary was the last outstanding
  pre-A1 criterion, and it no longer appears as a blocker.

  - **Chapter 328, *Never, Barely, Almost, Even*** — `así`, `jamás`, `apenas`,
    `casi`, `incluso`. A manner word and then a scale of degree, from none at
    all up to more than expected.
  - **Chapter 329, *Such, Which, Then, But Rather*** — `tal`, `cual`, `pues`,
    `sino`, `salvo`. Five joiners: qualify, ask the kind, conclude, correct a
    negative, carve out an exception.
  - **Chapter 330, *The Road and the World*** — `el camino`, `la parada`,
    `el mundo`, `la vida`, `la vez`.
  - **Chapter 331, *A While, a Way, a Thing*** — `el rato`, `la manera`,
    `el modo`, `la cosa`, `el asunto`.
  - **Chapter 332, *Permission, Notice, and the Word*** — `el permiso`,
    `el aviso`, `la disculpa`, `la palabra`, `la voz`.
  - **Chapter 333, *Noise, Silence, Calm and Hurry*** — `el ruido`,
    `el silencio`, `la calma`, `la prisa`, `el cuidado`.
  - **Chapter 334, *Health, Hunger, Cold and Heat*** — `la salud`, `el hambre`,
    `el frío`, `el calor`, `la edad`.

- **Eight payoffs on phrases the track already teaches.** Following the pattern
  `ES-C320-favor` set with *por favor*, several of these lessons open a noun the
  learner has been saying for hundreds of lessons without unpacking:
  *con permiso* yields `el permiso`, *otra vez* and *tal vez* yield `la vez`
  (and `tal` itself, one chapter earlier), *hace frío* and *hace calor* yield
  `el frío` and `el calor`, *sano y salvo* from chapter 327 yields `salvo`,
  *la pena* yields `apenas`, and *cómo* — from Latin *quōmodo* — turns out to
  have had `el modo` inside it since chapter 3.

- **Two real grammar rules, each given a whole lesson.** `el hambre` teaches
  why a feminine noun takes *el* before a stressed *a-* while staying feminine
  (*mucha hambre*, never *mucho hambre*), and carries the *f-* to *h-* shift
  that also produced *hacer*, *hijo*, *hierro* and *hermano*. `ES-C329-sino`
  teaches the distinction between *pero* (contrast) and *sino* (correction of a
  denial), which English collapses into one word.

- **Wiring** — seven `chapters.json` entries, seven `ES-PATH-328-01`…
  `ES-PATH-334-01` curriculum paths with one `extensions` entry per chapter,
  each path appended to its spine node's segments, and seven
  `core/book-generation.json` targets. Modality, narration, gentle-ramp
  snapshots, book chapters and track progress regenerated.

### Notes

- **Two allocated words were substituted after re-verification.** *acaso* was
  claimed by `ES-C324-acaso` (chapter 324, sequence 5340) in the tranche that
  landed immediately before this one, and *quizá* is the same lexeme as
  *quizás*, already taught at `ES-C271-quizas` (sequence 2740). Because
  `vocabularyOf` compares whole headword strings, neither would have failed the
  build — a near-duplicate silently **inflates** the gate instead. They were
  replaced with `casi` and `incluso`, both of which sit naturally on the same
  degree scale as the rest of chapter 328.

- **Root slugs are minted per etymon, never fused.** `el camino` records
  `cammanos-celtic` and the lesson explicitly separates it from Latin *camīnus*
  (a furnace, and the source of English *chimney*), which is a lookalike and not
  a relative. `la salud` records `salus-latin` distinct from the `salvus-latin`
  of `salvo`, and the lesson says plainly that the two are relatives from one
  ancient root rather than one word met twice.

- All seven chapters are ear-only: each chapter's narration opens *"All 5 can be
  done entirely by ear."*
