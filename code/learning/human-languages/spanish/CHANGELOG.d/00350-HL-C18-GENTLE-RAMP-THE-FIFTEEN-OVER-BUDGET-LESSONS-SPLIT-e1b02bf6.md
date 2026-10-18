## HL-C18 gentle ramp — the fifteen over-budget lessons, split

`core/chapter-policy.json` caps a lesson at `maxNewAtomsPerLesson: 3`, measured at
the corpus's own p90. Fifteen Spanish lessons exceeded it, led by
`ES-C31-numeros-11-20` at **seven** — ten numbers in one sitting, exactly the
"drill a list of ten greetings" model HL00 rejects. All fifteen are now split into
**thirty-three** prerequisite-ordered micro-lessons (net +18 lessons). Nothing was
waived and no atom was re-declared away: every atom the original taught is still
introduced, once, by whichever half now owns it.

Each boundary follows a seam in the language rather than an atom count:

| Was | Becomes | The seam |
|---|---|---|
| `ES-C31-numeros-11-20` (7) | `once-quince` · `dieciseis-diecinueve` · `teens-latinos` · `veinte` | 11–15 are **fused** Latin compounds you must remember; 16–19 are **transparent** *dieci-* + digit you can generate. Latin's own subtractive *duodēvīgintī*/*ūndēvīgintī* get their own lesson, and *veinte* was worn but never fused. |
| `ES-C30-el-tiempo` (6) | `el-tiempo` · `hace-calor` · `llueve` | *tiempo* the double-sense noun; then the **nouns** *calor/frío/sol* that need *hacer*; then *llueve*, the impersonal verb that refuses the *hacer* frame. |
| `ES-C03-tu-usted` (5) | `tu-usted` · `tu-usted-register` | The two pronouns, then the **register** choice and the third-person agreement that follows from *usted*'s origin as a noun phrase. |
| `ES-C06-hablar` (5) | `hablar` · `ar-presente` | One word and its *f-* → *h-* history, then the **productive** *-ar* paradigm and pro-drop. |
| `ES-C22-negro-blanco` (5) | `negro-blanco` · `blanco-germanico` | The two colour words, then the **borrowing event** and the retreat of Latin *albus*. |
| `ES-C26-agua-vino` (5) | `agua` · `vino` | *agua* carries the stressed-*a* article rule; *vino* carries none. |
| `ES-W02-enye` (5) | `enye` · `enye-formas` | The letter, sound and the scribal *-nn-*; then *mañana*/*español*, which reach ñ by the **other** road, Latin *-ni-*. |
| `ES-C03-como` (4) | `como` · `como-acento` | The word eroded from *quo modo*; then Spanish's **question-marking orthography** (¿ and the diacritic). |
| `ES-C04-estar` (4) | `estar` · `estar-estado` | The verb and *stāre*; then what *estar* is for and its two forms. |
| `ES-C06-espanol` (4) | `espanol` · `hablo-espanol` | The noun from *Hispania*; then the assembled sentence and the bare-language rule. |
| `ES-C20-lo-siento` (4) | `lo-siento` · `perdon` | Two different words, two different roots, two different pragmatic jobs. |
| `ES-C22-rojo-azul` (4) | `rojo` · `azul` | Inherited via *russus*; borrowed from Arabic *lāzaward*. |
| `ES-C23-hermano-hermana` (4) | `hermano-hermana` · `hermano-hache` | The *germen* etymology; then the silent *h*, which is **not** the *hijo*/*hacer* *h*. |
| `ES-C32-perro-gato` (4) | `gato` · `perro` | A fully sourced etymology against a genuinely unsolved one. |
| `ES-C33-verde-amarillo` (4) | `verde` · `amarillo` | Named after a colour verb; named after a **taste**. |

Consequences:

- Five paired lessons were renamed to single-word ids (`ES-C22-rojo`, `ES-C26-agua`,
  `ES-C31-once-quince`, `ES-C32-gato`, `ES-C33-verde`), because keeping a two-word id
  on a one-word lesson would have been a lie in the filename.
- `curriculum.json` gains the eighteen new lessons in prerequisite-safe positions and
  four new extension nodes (`ES-EXT-007-REGISTER`, `ES-EXT-008-SCRIPT`,
  `ES-EXT-011-GRAMMAR`, `ES-EXT-018-LANGUAGE-SPECIFIC`).
- `chapters.json` repoints seven payoffs at the real terminal lesson: Chapters 20, 22,
  23, 30, 31, 32 and 33. Each `assesses` list is still a subset of that lesson's own
  `practises.knowledge`.
- Every split lesson computes **well under 300 effective seconds** (maximum 275,
  `ES-C03-como-acento`), and each declared `max_seconds` is at least the computed cost.
- Spanish over-budget count: **15 → 0**. Corpus-wide: 52 → 37, with the maximum
  dropping from 7 to 6.

