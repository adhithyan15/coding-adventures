# Changelog

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*nanna hesaru … / nimma hesaru
  ēnu?*), atom-first, Kannada inline (`lessons/KA-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom traced:
  - **ಹೆಸರು** hesaru ("name") ← Proto-Dravidian *\*pesar*, via Kannada's **p→h**
    shift — cousin of Tamil *peyar*, **not** the Indo-European *name/nām*.
  - **ನನ್ನ** nanna ("my") ← *nānu* ("I").
  - **ನನ್ನ ಹೆಸರು …** — **"my name is…"**; the **zero copula** (no "is").
  - **ನೀನು / ನೀವು** nīnu/nīvu — "you," familiar/respectful; respect by plural.
  - **ಏನು** ēnu ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **ನಿಮ್ಮ ಹೆಸರು ಏನು?** — **"what's your name?"**
  - **ಸಂತೋಷ** santōṣa — "pleased to meet you," a **Sanskrit** loan (vs. Tamil's
    native *magiḻcci*) — the Kannada-borrows-Sanskrit thread continued.
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Kannada script taught inline)

- New Kannada track on the HL00 framework — the second of the four Dravidian
  tracks, after the Tamil anchor. One word per lesson, slug ids, atom-first,
  derivations shown, LaTeX book. Uses the **vendored** Noto Sans Kannada font
  (loaded by relative `Path=` so local and CI builds match; script shaped via
  `Script=Kannada`, no polyglossia language module needed).
- **No reading course.** Per `HL00`'s inline-letters rule, Kannada is taught
  *inside* each word lesson: a *"The letters in this word"* section introduces
  exactly the letters that word needs, so reading and meaning arrive together.
- Chapter 1 (`lessons/KA-C01-*`), greetings + conversational glue:
  - **ನಮಸ್ಕಾರ** namaskāra ("hello," **Sanskrit** *namas* + *kāra*) — teaches
    the inherent *a*, vowel signs, and the ಸ್ಕ *ottakṣara* (stacked conjunct).
  - **ಧನ್ಯವಾದ** dhanyavāda ("thanks," **Sanskrit** *dhanya* + *vāda*) — the
    aspirated ಧ and the ನ್ಯ conjunct.
  - **ಹೌದು** haudu ("yes," native) — the au/u vowel signs; verb-echo "yes."
  - **ಇಲ್ಲ** illa ("no / isn't," native, root *il*) — cognate with Tamil
    *illai*; negation by a negative existential verb.
  - **ಸರಿ** sari ("okay," native) — the *same word* as Tamil *sari*, one word
    in two scripts.
  - **practice** — recap + the *hōgi baruttēne* / *hōgi banni* farewell (the
    same "go and come back" logic as Tamil's *pōy varugiṟēṉ*).
- The recurring thread: **Kannada borrowed Sanskrit for greetings/politeness
  (namaskāra, dhanyavāda) but kept native Dravidian for the everyday grammar
  (haudu, illa, sari)** — each lesson carries an "Across the family" cognate box
  (English / Sanskrit / Hindi / Tamil / Telugu / Malayalam), every form
  supplied so nothing is assumed. Book compiles clean with XeLaTeX.
