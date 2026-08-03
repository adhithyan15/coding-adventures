# Changelog

## Sub-five-minute lesson remediation (2026-08-02)

- All thirty-seven Malayalam duration violations are resolved. Thirty-three
  lessons already computed below five minutes and now declare an honest
  four-minute budget without changing their teaching content.
- Four long lessons become gentle prerequisite pairs: **ഉച്ച** noon →
  **പാതിരാ** midnight; Sanskrit *divasam/dinam* → native **നാൾ**; Sanskrit
  **രാത്രി** → native *iravŭ/iruḷ*; formal **ശുഭ മധ്യാഹ്നം** → the three-language
  convergence map. The eight steps compute between 141 and 235 seconds.
- The four support lessons bring the Malayalam track to 64 lessons with zero
  unknown prerequisite ids; downstream lessons now require the moved concept
  before using it.
- A forced book build succeeds at 31 pages with no missing glyphs. Canonical
  lessons continue through Chapter 31 while the book stops at Chapter 5
  (`HL-B26`); existing layout, bookmark, duplicate-label, and font warnings are
  tracked in `HL-B27`; roadmap and session-map drift is tracked in `HL-M04`.

## Chapter 6 — Case endings, and the sentence with no subject

- **Chapter 6 authored** (`ML-C06-dative-ikku`, `-dative-subject`): the track's
  first **case ending** — reviewing Ch.2/3/5 via `reviews_of`.
- **-ിക്ക്/-ിന്** (`ML-C06-dative-ikku`): the dative "to/for," taught as the doorway
  to **agglutination**. Malayalam **adds** a suffix carrying **one** meaning with
  the **seam visible** (*jōli* + *kku*), where a Latin ending like *-īs* **fuses**
  case+number+declension inseparably; the two shapes are **one case** chosen by the
  noun's ending. Includes *ñān* → **എനിക്ക്** *enikku*, flagged as worth memorising
  cold — it opens a great many everyday Malayalam sentences.
- **എനിക്ക് മലയാളം അറിയാം** (`ML-C06-dative-subject`): "I know Malayalam" — literally
  "**to-me Malayalam is-knowable**" — *aṟiyām* being *aṟiy-* "know" plus the
  **abilitative** *-ām*, not a passive — with **no nominative "I"** (contrast Ch.5's
  *ñān malayāḷam saṁsārikkunnu*). Explains the **dative-subject** rule with
  English's "**methinks**" as the bridge.
- **The Dravidian family thread**, new in this chapter: *-ikku / -ukku / -ku / -ge*
  are visibly the **same suffix**, with the extra observation that **Malayalam's
  *enikku* and Tamil's *enakku* are nearly the same word** — the two languages
  separated most recently of the four, and it shows.
- Taxonomy: namespaced `ML-CASE-DATIVE`, `ML-DATIVE-SUBJECT`.

## Chapters 3–5 — How-are-you, Farewells, First Verbs

- Three new chapters carry Malayalam to Chapter 5, matching the leading tracks'
  arc. One word per lesson, atom-first, Malayalam script inline; every root traced
  (`lessons/ML-C0{3,4,5}-*`, `book/chapters/ch0{3,4,5}-*.tex`). Concept tags reuse
  the universal `HL01` taxonomy; verbs namespaced (`ML-VERB-*`). Malayalam's
  double character — Tamil's closest sister, yet the deepest in Sanskrit, and the
  only one with a real copula — runs throughout.
- **Ch. 3 — How Are You**: *eṅṅane* (how; the native *e-* questions) → *sukhamāṇō?*
  ("are you well?" — the Ch.2 copula *āṇŭ* + the question particle *-ō*) → *ñān*
  (I ← Proto-Dravidian; **can't be dropped**, since Malayalam verbs don't mark
  person) → *sukham* (well ← Sanskrit *sukha*, the *su-* that is Greek *eu-*) →
  *sāramilla* ("no matter" = you're welcome; Sanskrit *sāraṁ* + native *illa*) →
  practice.
- **Ch. 4 — Farewells**: *pōkuka*/*varika* → *pōyi varāṁ* ("I'll go and come back,"
  tabled across the family) → *nāḷe kāṇāṁ* (see you tomorrow; *nāḷ* "day" + *kāṇ*
  "see" + the "let's" *-āṁ*) → *vīṇḍuṁ kāṇāṁ* (we'll meet again; native *kāṇ*,
  where Tamil borrowed Sanskrit *sandi*) → practice.
- **Ch. 5 — First Verbs**: *saṁsārikkuka* (Sanskrit-derived; native twin
  *paṟayuka*) → *ñān malayāḷaṁ saṁsārikkunnu* (I speak Malayalam; the *-unnu*
  present — **the verb never changes for person**, Malayalam's great
  simplification) → *tāmasikkuka* (to live; postposition *-il*) → *jōli ceyyuka*
  (to work; *ceyyuka* is the *same root* as Tamil *sey*) → practice. Book compiles
  clean with XeLaTeX (0 missing chars, 0 undefined refs).

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*enṟe pēru … āṇŭ / ninṟe pēru
  entāṇŭ?*), atom-first, Malayalam inline (`lessons/ML-C02-*`,
  `book/chapters/ch02-introductions.tex`). Every atom traced:
  - **പേര്** pēru ("name") ← Proto-Dravidian *\*pēr* — twin of Tamil *peyar*,
    **not** the Indo-European *name/nām*.
  - **എന്റെ** enṟe ("my") ← *ñāṉ* ("I").
  - **ആണ്** āṇŭ ("is") — Malayalam's **copula**, from the verb *āka*. The
    standout: Tamil/Kannada/Telugu use the **zero copula**, but Malayalam,
    Tamil's closest sister, grammaticalised a "to be" verb.
  - **എന്റെ പേര് … ആണ്** — **"my name is…"**; verb last (unlike Tamil).
  - **നീ / നിങ്ങൾ** nī/niṅṅaḷ — "you," familiar/respectful; respect by plural.
  - **എന്ത്** entŭ ("what") ← Dravidian question-stem *\*yā-/\*e-*.
  - **നിന്റെ പേര് എന്താണ്?** — **"what's your name?"** (*entŭ* + *āṇŭ* fused).
  - **സന്തോഷം** santōṣam — "pleased to meet you," a **Sanskrit** loan (Malayalam
    borrows selectively: native *nandi* for thanks, Sanskrit here).
  - **practice** — the whole dialogue.
- Example names are invented (Mira / Arun). Book compiles clean with XeLaTeX.

## Chapter 1 — Greetings (Malayalam script taught inline)

- New Malayalam track on the HL00 framework — the last of the four Dravidian
  tracks. One word per lesson, slug ids, atom-first, derivations shown, LaTeX
  book. Uses the **vendored** Noto Sans Malayalam font (relative `Path=`, shaped
  via `Script=Malayalam`, no polyglossia language module needed).
- **No reading course.** Per `HL00`'s inline-letters rule, Malayalam is taught
  *inside* each word lesson.
- Chapter 1 (`lessons/ML-C01-*`), greetings + conversational glue:
  - **നമസ്കാരം** namaskāram ("hello," **Sanskrit** namas + kāra) — inherent
    *a*, vowel signs, the chandrakkala, the സ്ക conjunct, anusvāram ം.
  - **നന്ദി** nandi ("thanks," **native**, root *nal*) — the twin of Tamil
    *naṉṟi*; the ന്ദ conjunct.
  - **അതെ** athe ("yes," native, "that [is so]") — yes/no as demonstratives;
    the *e*-sign written before its consonant.
  - **ഇല്ല** illa ("no / isn't," native, root *il*) — the twin of Tamil
    *illai*; negation by a negative existential verb.
  - **ശരി** śari ("okay," native) — the family word *sari* with Sanskrit ശ.
  - **practice** — recap + the *pōyi varām* farewell (nearly the same words as
    Tamil's *pōy varugiṟēṉ*).
- The recurring thread: **Malayalam is Tamil's closest sister** — four of the
  five everyday words are shared with Tamil (nandi, athe, illa, śari) — **with a
  heavy Sanskrit overlay** (namaskāram; the largest alphabet in the family).
  Each lesson carries an "Across the family" cognate box (English / Sanskrit /
  Hindi / Tamil / Kannada / Telugu), every form supplied so nothing is assumed.
  Book compiles clean with XeLaTeX. Completes the four Dravidian first chapters.
