# Changelog

## Chapter 6 — Case endings, and the sentence with no subject

- **Chapter 6 authored** (`KA-C06-dative-ge`, `-dative-subject`): the track's first
  **case ending** — reviewing Ch.2/3/5 via `reviews_of`.
- **-ಗೆ** (`KA-C06-dative-ge`): the dative "to/for," taught as the doorway to
  **agglutination**. Kannada **adds** a suffix carrying **one** meaning with the
  **seam visible** (*hesar* + *ige*), where a Latin ending like *-īs* **fuses**
  case+number+declension inseparably. The distinctive Kannada content is a
  **sound-history** point: its **g** is the shared Dravidian dative ***k* softened
  between vowels** — a voicing Kannada carried **into the dative** where its
  sisters kept the hard consonant — while the
  *-kke* form (*kelasakke*) preserves the original hard doubled consonant. Includes
  *nānu* → **ನನಗೆ** *nanage*.
- **ನನಗೆ ಕನ್ನಡ ಗೊತ್ತು** (`KA-C06-dative-subject`): "I know Kannada" — literally
  "**to-me Kannada known**," with **no nominative subject** (contrast Ch.5's *nānu
  kannaḍa mātanāḍuttēne*), and with the further observation that ***gottu* isn't an
  action verb at all** — so nothing in the sentence is *doing* anything. Explains
  the **dative-subject** rule with English's "**methinks**" as the bridge.
- **The Dravidian family thread**, new in this chapter: *-ge / -ukku / -ku / -ikku*
  are visibly the **same suffix** across the four sisters, all building "I know X"
  the same subjectless way.
- Taxonomy: namespaced `KA-CASE-DATIVE`, `KA-DATIVE-SUBJECT`.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Kannada to Chapter 5, matching the leading tracks' arc.
  One word per lesson, atom-first, Kannada script inline; every root traced
  (`lessons/KA-C0{3,4,5}-*`, `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse
  the universal `HL01` taxonomy; verbs namespaced (`KA-VERB-*`). The
  Sanskrit-borrowing-yet-Dravidian-grammar thread runs throughout.
- **Ch. 3 — How Are You**: *hēge* (how; the native *ē-/yā-* questions) → *nīvu
  hēgiddīrā?* (the verb *iru* "to be," the same word Tamil uses) → *nānu* (I ←
  Proto-Dravidian, unrelated to *me*) → *cennāgi* (well ← *cennu* "beautiful";
  *nānu cennāgiddēne* "I'm well") → *paravāgilla* ("no harm" = you're welcome, on
  the Dravidian *illa* shared with Tamil/Malayalam — where Telugu uses *lēdu*) →
  practice.
- **Ch. 4 — Farewells**: *hōgu*/*bā* → *hōgi baruttēne* ("I'll go and come back,"
  tabled across the family) → *nāḷe sigōṇa* (see you tomorrow; *nāḷ* "day" shared
  with Tamil, the "let's ___" *-ōṇa*) → *matte sigōṇa* (we'll meet again; native
  *sigu*, where Tamil borrowed Sanskrit *sandi*) → practice.
- **Ch. 5 — First Verbs**: *mātanāḍu* (← *mātu* "word") → *nānu kannaḍa
  mātanāḍuttēne* (I speak Kannada; no 1st-person gender) → *iru* (to be/stay/live;
  the postposition *-alli*) → *kelasa māḍu* (to work; noun + *māḍu*, the twin of
  Hindi's *karnā*) → practice. Book compiles clean with XeLaTeX (0 missing chars,
  0 undefined refs).

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
