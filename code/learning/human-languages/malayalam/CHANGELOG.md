# Changelog

## Chapters 33-34 — The eight verbs (2026-08-07)

- Added eight schema-v2 lessons carrying the eight canonical concepts fifteen
  other tracks already teach — `VERB-THINK`, `VERB-UNDERSTAND`, `VERB-READ`,
  `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP`, `VERB-LIKE-LOVE` — making
  Malayalam the sixteenth track to hold all of them, and the fourth Dravidian
  contributor after Tamil, Kannada and Telugu.
- **Two chapters of four, never one of eight.** Chapter 33 introduces 9 atoms
  and Chapter 34 introduces 9, both under the `maxNewAtomsPerChapter` budget of
  12 that one chapter of eight would have blown. Each carries its own `canDo`
  and its own payoff, and both payoffs assess **every** atom their chapter
  introduces — 9/9 and 9/9, representativeness **1.00** against the 0.5 floor.
- **Ch. 33 (The Mind and the Page)**: ചിന്തിക്കുക, മനസ്സിലാക്കുക, വായിക്കുക,
  എഴുതുക. Its spine is a diagnostic the learner can hear: the **ഇ** of
  **‑ഇക്കുക**, and the **‑ഇച്ചു** past that goes with it, mark a Sanskrit
  borrowing. മനസ്സിലാക്കുക comes apart into *manassŭ* + locative *‑il* +
  *ākkuka* — "to put in the mind" — and its intransitive twin **എനിക്ക്
  മനസ്സിലായി** puts the understander in the dative, joining Ch. 6's *enikku
  malayāḷaṁ aṟiyāṁ*. എഴുതുക is Tamil **எழுது** unchanged, carrying the **ഴ**
  that Kannada and Telugu lost — and with it the verb, which is why they write
  with the line-drawing root Malayalam kept for വരയ്ക്കുക, "to draw."
- **Ch. 34 (Taking, Asking, Helping, Liking)**: എടുക്കുക, ചോദിക്കുക,
  സഹായിക്കുക, എനിക്ക് മലയാളം ഇഷ്ടമാണ്. എടുക്കുക corrects Ch. 32's rule rather
  than breaking it — the stamp is the **ഇ**, not the doubled **ക്ക** a native
  stem carries by itself. ചോദിക്കുക is Sanskrit *cud*, "to urge," and the
  reason a loan was needed at all is structural: Proto-Dravidian *\*kēḷ‑*
  covered hearing **and** asking, Tamil's கேள் still does both, and Malayalam
  narrowed കേൾക്കുക to hearing alone. The same shape recurs four times across
  the two chapters — ഓതുക, ഓർക്കുക, കേൾക്കുക, ഉതവി each gave up an everyday
  slot to a Sanskrit word — which is the Maṇipravāḷam era's lexical footprint.
- **Every etymology was checked against sources rather than taken on trust, and
  four briefed or inherited claims were corrected in the process.**
  - വായിക്കുക is **not** from വായ് "mouth". Gundert marks വായന a *tadbhava* of
    Sanskrit *vac* and gives the Tamil verb as *vācikka*; DEDR 5352 (*vāy*
    "mouth") carries no reading sense in any Dravidian language. The mouth story
    is folk etymology, and the lesson says so.
  - The *c* → *y* is **not** a Malayalam sound law. It happened in the middle
    Indo-Aryan stage the *tadbhava* passed through; Malayalam holds the same
    Sanskrit word borrowed straight as വാചകം, so the doublet is visible in the
    language itself.
  - എഴുതുക is **not** filed under എഴു "to rise". Burrow–Emeneau keep them as
    separate entries, so the lesson names the link and marks it unproven — while
    എടുക്കുക genuinely *is* filed inside the rise-family, which is where that
    fact belongs.
  - English *mind* descends from *\*ménti‑*, not from the *\*ménos* that gave
    Sanskrit मनस्; the lesson claims root-level cognacy only, and names मति and
    Greek *ménos* as the exact matches.
  Two further claims are hedged rather than asserted: Monier-Williams marks the
  *saha* + *aya* reading of सहाय "probable", and whether Kannada/Telugu *ettu*
  is the cognate of എടു is a point Gundert and DEDR disagree on.
- **Reinforced at two cadences.** Every lesson's `practises.knowledge` names
  atoms from the immediately preceding one to three lessons, across the chapter
  seam; each payoff reaches several chapters back. Malayalam's never-revisited
  atoms fall from **72 of 78 (92%)** to **46 of 96 (48%)** — 29 previously
  orphaned atoms rescued, spanning Chapters 6, 7, 8, 9, 10, 13, 15, 16, 18, 19,
  20, 24, 26 and 32. The three that remain from this tranche belong to the
  final lesson of the track, which no later lesson exists to retrieve.
- All eight use the canonical `## The letters in this word` heading. That block
  classifies as `script`, which is **detachable**, so every lesson derives
  `modality: sight` with `coreModality: voice` — the driving edition is intact.
  (`core/lesson-modality.json` reports `drivable` from the whole-lesson modality
  rather than the core, so the published manifest understates this; that is a
  known bug in `modality-manifest.ts`, not a property of these lessons.)
- All eight sit under the 300-second effective ceiling (285-299s computed).
- Wiring: `ML-PATH-027`/`ML-PATH-028` and `ML-EXT-027-MIND-VERBS`/
  `ML-EXT-028-DOING-VERBS` in [`curriculum.json`](./curriculum.json), all eight
  concepts dropped from the `SPINE-SAY-WHAT-I-DO` omission ledger (36 omits down
  to 28), Chapter 33 and 34 ledger entries in [`chapters.json`](./chapters.json),
  two `core/book-generation.json` targets, the generated
  `book/chapters/ch33-mind-and-page.tex` and
  `book/chapters/ch34-taking-asking-helping-liking.tex`, `\input` in `book.tex`,
  and generated narration for both chapters.
- The 136-page XeLaTeX build has **zero missing characters** and zero errors.
  Three underfull boxes remain, two of them pre-existing in Chapters 6 and 30;
  the third is the Chapter 34 payoff's section heading, where the Malayalam
  script at 14.4pt forces an awkward break. The track is `null` in
  `core/latex-warning-baseline.json`, so nothing is re-pinned.

## Chapter 32 — The Core Verbs (2026-08-06)

- Added six schema-v2 core-verb lessons, the track's first A2 material and its
  first realization of `SPINE-SAY-WHAT-I-DO`: `ML-C32-undu` (VERB-BE),
  `ML-C32-pokuka` (VERB-GO), `ML-C32-varuka` (VERB-COME), `ML-C32-tinnuka`
  (VERB-EAT), `ML-C32-kaanuka` (VERB-SEE), `ML-C32-ariyuka` (VERB-KNOW). All
  six take canonical spine concept tags, so the track goes from four namespaced
  verb concepts and none canonical to six canonical ones.
- The chapter is built around the fact that makes Malayalam unlike its three
  Dravidian sisters: **its verb carries no person marking at all**. Chapter 5
  observed this for one verb; Chapter 32 turns it into the chapter's spine.
  `ML-C32-undu` sets up the two-slot machine (stem + tense) against Tamil's
  three (stem + tense + person); `ML-C32-pokuka` shows that each tense form is
  therefore the *whole* conjugation; `ML-C32-varuka` locates the entire
  irregularity budget in the past (*varu-* → *vann-*); `ML-C32-kaanuka` shows
  the freed slot spent on mood (*kāṇāṁ*, *kāṇaṇaṁ*, *kāṇarutŭ*); and
  `ML-C32-ariyuka` closes by taking the person out of the subject slot too,
  giving Chapter 6's **എനിക്ക് മലയാളം അറിയാം** the verb it was always built on.
- Two genuinely conservative facts are recorded rather than glossed: Malayalam
  kept **both** of the family's be-verbs (ഉണ്ട് = Telugu ఉండు for existing and
  having, ഇരിക്കുക = Tamil இரு / Kannada ಇರು for being somewhere), and it kept
  the inherited *tiṉ-* and *kāṇ-* as its everyday eat- and see-words where
  Tamil, Kannada and Telugu each moved on. `ML-C32-tinnuka` also names the
  **-ഉക / -ഇക്കുക** split, which marks a native verb off from a Sanskrit
  borrowing turned into one.
- Every non-Malayalam form is supplied in full — no lesson assumes the reader
  knows another target language — and every cognate claim stays inside
  Dravidian, with Sanskrit material flagged as borrowing.
- Wired the chapter through the pipeline: `ML-PATH-026` in
  [`curriculum.json`](./curriculum.json) (dropping the six concepts from
  `SPINE-SAY-WHAT-I-DO`'s `omits`), a Chapter 32 ledger entry in
  [`chapters.json`](./chapters.json), a `core/book-generation.json` target, the
  generated `book/chapters/ch32-core-verbs.tex`, and `\input` in `book.tex`.
- All six lessons are **voice** modality, so Chapter 32 is drivable end to end,
  and all six sit under the 300-second effective ceiling (272–294s computed).
  The 114-page XeLaTeX build has zero missing characters and adds no over- or
  underfull boxes.

## Chapter capability ledger for Chapters 6–31 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 chapter capability
  ledger: one `canDo` promise and one validated payoff for each of Chapters
  6–31. Titles and labels are copied from `core/book-generation.json` so the two
  agree until HL-C04 inverts that dependency; `spineNodes` are derived from
  `curriculum.json`'s path segments; every `payoff.assesses` atom is taken from
  the payoff lesson's own `practises.knowledge`, never invented.
- Derived from the lessons and `curriculum.json` rather than from
  [`roadmap.md`](./roadmap.md) or [`session-map.md`](./session-map.md), which
  still lag the canonical Chapters 6–31 (known debt, HL-M04).
- **Chapters 1–5 are deliberately absent.** Their terminal practice lessons
  (`ML-C01-practice` … `ML-C05-practice`) are still schema v1 and declare no
  `practises.knowledge`, so no payoff can name an atom without fabricating one.
  Those five chapters also have no `book-generation.json` target to copy a title
  from. The gap is recorded in the file's own `note` and stays visible to the
  HL05 gap report rather than being filled with a placeholder.
- No chapter from 6 on ends in a `practice` lesson, so every payoff is that
  chapter's last lesson by `sequence`. Where that terminal lesson is an
  `etymology` lesson (Chapters 23, 24, 31) the payoff is typed `task` and its
  summary describes the sorting the reader actually does.

## Warning-free complete book (2026-08-03)

- Added explicit static bold and italic faces for Malayalam and every
  comparison script, plus bookmark-safe Unicode commands, eliminating all
  font-shape and Hyperref warnings without dropping multilingual examples.
- Made the five handwritten recap labels unique and shortened only the running
  titles that exceeded the text block. Small sentence-level copy-flow repairs
  in the generated family, number, and colour chapters remove the remaining
  horizontal overflows while preserving the canonical teaching sequence.
- Added natural page bottoms for deliberately short micro-lessons and made
  open-right chapter versos truly empty, without a running header or page
  number.
- The forced 107-page build now has zero missing glyphs, overfull or underfull
  boxes, duplicate destinations, Hyperref warnings, LaTeX warnings, or font
  warnings. All 107 pages were rendered and visually inspected.
- The 33 top-level and 97 total outline entries, title and author metadata,
  generated source hashes, and zero schema or generator leaks remain intact.

## Canonical Chapters 6–31 in the book (2026-08-03)

- Migrated all thirty-three Malayalam lessons after Chapter 5 to the strict
  schema-v2 curriculum contract: canonical spine nodes, unique
  prerequisite-safe sequence, explicit sub-five-minute budgets, typed block
  boundaries, and closed knowledge introductions and assessments.
- Generated twenty-six LaTeX chapters from those canonical lessons instead of
  copying app content into a separate book source. The committed source-hash
  manifest is independently checked against Language Ladder for Chapters 6–31.
- Added a reusable Malayalam comparison-font set for Malayalam, Tamil, Telugu,
  Kannada, Devanagari, and Arabic-script examples. The 107-page PDF has zero
  missing glyphs and preserves the full 33-entry top-level chapter outline.
- Rendered and inspected all 107 pages, including dense case, calendar,
  etymology, daypart, and register sections. No teaching content is clipped,
  colliding, accidentally omitted, or replaced by generator metadata.
- The expanded artifact's cleanup baseline is 17 overfull boxes, four
  underfull horizontal boxes, ten underfull vertical boxes, four duplicate
  practice labels, 108 Hyperref warnings, and seven font warnings. `HL-B27`
  tracks those warnings and the running headers on intentionally empty versos.
- The single all-books publication gate still compiles and catalogs all twenty
  downloadable volumes successfully.

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
