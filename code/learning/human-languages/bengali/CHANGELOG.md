# Changelog

## Chapters 8 and 9 — the eight-verb tranche — 2026-08-07

- Authored **eight** schema-v2 lessons in **two** chapters of four, realizing the
  canonical `SPINE-SAY-WHAT-I-DO` concepts `VERB-THINK`, `VERB-UNDERSTAND`,
  `VERB-READ`, `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP` and
  `VERB-LIKE-LOVE`. Each of the eight was taught by exactly **three** tracks
  before this (Spanish, Latin, Portuguese) and is now taught by **four**. Bengali
  goes from **6 of 40** core verbs to **14 of 40**.
- **Chapter 8 — The Mind and the Page**, 4 lessons, **8** new atoms:
  - **ভাবা** *bhābā* — Sanskrit *bhāvayati* is the **causative** of √bhū, which
    is Chapter 7's হওয়া. Thinking is making something be. The gear is still
    live in Bengali as **-আনো**: দেখা → দেখানো "show," খাওয়া → খাওয়ানো "feed."
  - **বোঝা** *bojhā* — √budh "to wake," the root that titled the **Buddha**;
    PIE *\*bʰewdʰ-* → English **bid**, **forbid**. Vowel harmony returns on a new
    vowel, and this time the **spelling moves**: বুঝি against বোঝে. Three
    knowings now stand where English has one — জানা, চেনা, বোঝা.
  - **পড়া** *pôṛā* — √paṭh "to recite aloud." A single intervocalic retroflex
    softens into **ড়**, which is why the letter exists at all; the same
    softening dragged √pat "to fall" onto the identical spelling, and it is the
    **falling** twin that owns **feather**, **petition** and **pterodactyl**.
  - **লেখা** *lekhā* — √likh "to scratch," beside Latin *scrībere* and Germanic
    *wrītan*, both also "scratch": three unrelated roots, one idea, named as
    **convergence and not kinship**. And every **-া** form is simultaneously a
    **noun**, which is what Chapter 4's *dækhā hôbe* had been doing all along.
- **Chapter 9 — Taking, Asking, Helping, Liking**, 4 lessons, **9** new atoms:
  - **নেওয়া** *neowā* — √nī "to lead." Its working life is as the verb that
    **closes a compound**: লিখে নেওয়া "write it down," নিয়ে আসা "bring"
    (Bengali has no separate word for it), নিয়ে যাওয়া "take away."
  - **জিজ্ঞাসা করা** *jijñāsā kôrā* — জিজ্ঞাসা is the Sanskrit **desiderative**
    of √jñā, so asking is literally *wanting to know*: Chapter 7's জানা with an
    appetite, on the same PIE *\*ǵneh₃-* that gives English **know**. And **noun
    + করা** is not a compound but *the* way Bengali makes verbs — the door this
    lesson opens is wider than the word.
  - **সাহায্য করা** *sāhājjo kôrā* — *sahāya*, "a companion," is **সহ-**
    "together" + **√i** "to go"; both cousins are secure (PIE *\*sem-* → **same**,
    Greek *homo-*; PIE *\*h₁ey-* → Latin *īre* → **exit**, **transit**). The
    doubled **য্য** finally demonstrates the word-final **inherent o** that
    Chapter 6 had to admit its five numerals could not show.
  - **ভালো লাগা** *bhālo lāgā* — √lag "to attach." *Āmār bhālo lāge* is "good
    sticks **to me**": the liker is not the subject, the same inversion Spanish
    makes with *gustar*. It wears the identical clothes as আমার … আছে, "I have."
    Set against **ভালোবাসা**, where you *are* the subject — the chapter's payoff
    is the contrast.
- **Honest dead ends, again named rather than papered over**: √nī left no living
  English descendant; √paṭh has no secure Indo-European pedigree past Sanskrit;
  and ভালোবাসা's *bāsā* half has a **disputed** origin, so the commonest proposal
  (√vas "to dwell," which would make English **was** its cousin) is reported as a
  proposal and left open.
- **Reinforcement at two cadences**, which is the point of splitting this into
  two chapters rather than one. Every lesson's `practises.knowledge` names atoms
  from the one to three lessons immediately before it, across the chapter seam;
  the two payoffs reach several chapters back. Measured result: Bengali's
  never-revisited atom count falls from **12 of 18** to **4 of 35** — and all
  twelve of the previously orphaned atoms (Chapter 6's six and Chapter 7's six)
  are now genuinely practised, not merely listed. The four that remain are the
  three introduced by the track's final lesson, which nothing can follow, and
  the doubled **য্য** of সাহায্য, which is recorded rather than claimed.
- Windows closed: **R1** for both দেখা atoms and both জানা atoms; **R2** for all
  six Chapter-6 atoms and all six Chapter-7 pairs. R1 for Chapters 6 and 7 was
  already out of reach — those windows close at reading positions 31–37, which
  are lessons this tranche does not edit — and that residue is left visible.
- Wired into `curriculum.json` (`BN-PATH-012`/`BN-PATH-013`,
  `BN-EXT-012-MIND-AND-PAGE`/`BN-EXT-013-CONJUNCT-VERBS`, and the eight concepts
  struck from the `SPINE-SAY-WHAT-I-DO` omission ledger), `chapters.json`,
  `core/book-generation.json`, `book/book.tex`, and the generated narration.
  All 45 Bengali lessons are on a path; none is orphaned.
- All eight use the canonical **`## The letters in this word`** heading, which
  types as a `script` block. That labels them `sight` and **detachable**, so
  every one has a `voice` core: the track's drivability rises to **98%**, with
  30 lessons rescued for the hands-free view. Every table is 2 or 3 columns; no
  lesson contains a sight cue. Computed durations **256–298 s**, all inside the
  300 s ceiling.
- The forced nine-chapter XeLaTeX build is **warning-free**: 54 pages, zero
  `Missing character`, zero over/underfull boxes. The new conjuncts — **ড়**,
  **জ্ঞ**, **য্য**, **দ্বার** — all render from the vendored Noto Sans Bengali
  with no preamble change.

## Chapter 7 — The Core Verbs — 2026-08-06

- Authored six schema-v2 lessons realizing the canonical `SPINE-SAY-WHAT-I-DO`
  concepts `VERB-BE`, `VERB-GO`, `VERB-COME`, `VERB-EAT`, `VERB-SEE` and
  `VERB-KNOW`. Before this the track realized **no** canonical verb concept: its
  only four verbs (*bôlā*, *thākā*, *kôrā*, *dækhā hôbe*) were all namespaced
  `BN-VERB-*` and none of them was on the shared spine.
- One idea per lesson, each one a thing Bengali does that its neighbours do not:
  - **হওয়া** *hôwā* — Bengali has **two** be-verbs, and আছ- is unfinished: it
    has a present and a past and nothing else, so the future falls to *hôbe* or
    to Chapter 5's থাকা. Root: Sanskrit √bhū, PIE *\*bʰuH-* → English **be**,
    **been**, **future**, **physics**.
  - **যাওয়া** *jāwā* — the honorific level lives in the **verb ending**:
    *jāsh* / *jāo* / *jān* for তুই / তুমি / আপনি, and *se jāy* against *tini
    jān* in the third person. Drop the pronoun and the register still stands.
  - **আসা** *āsā* — **no grammatical gender, anywhere**, set against Hindi
    *ātā/ātī*, Marathi *yeto/yete* and Gujarati's *āvyo/āvī* past. Not a
    beginner's simplification the grammar takes back later.
  - **খাওয়া** *khāwā* — Bengali **eats its drinks**: *jôl khāwā*, *chā khāwā*,
    where Hindi keeps *pīnā*. The formal পান করা carries √pā → English
    **potion**, **potable**.
  - **দেখা** *dækhā* — **vowel harmony**: *dekhi* closes where *dækhe* and
    *dækho* stay open, and the spelling দে never moves. Root: Sanskrit √dṛś, PIE
    *\*derḱ-* → Greek *drákōn* → English **dragon**.
  - **জানা** *jānā* — জানা for facts against চেনা for people, the *savoir* /
    *connaître* line English lost. Root: √jñā, PIE *\*ǵneh₃-* → **know**,
    **notice**, **diagnosis**.
- Flagged two dead ends honestly rather than inventing cousins: যাওয়া's PIE
  *\*yeh₂-* has no living English descendant, and খাওয়া's *khād-* has no secure
  Indo-European pedigree outside Indo-Aryan.
- All six derive as **`voice`** — the chapter's `drivablePrefix` is 6, every
  table is two columns, and no lesson leans on a sight cue. Computed durations
  257–281 s, all inside the 300 s ceiling.
- Wired the chapter into `curriculum.json` (`BN-PATH-011`,
  `BN-EXT-011-CORE-VERBS`, and six concepts struck from the
  `SPINE-SAY-WHAT-I-DO` omission ledger), `chapters.json` (payoff
  `BN-C07-jana`, 8/12 introduced atoms = 0.67, above the 0.5 floor),
  `core/book-generation.json`, and `book/book.tex`.
- Gave the book preamble an optional `grammarlens` title — the generator passes
  each lesson's own "Grammar Lens: …" heading through, which the old
  no-argument box could not accept — plus composed glyphs for the PIE palatals
  `ǵ` and `ḱ`. The seven-chapter build is still warning-free, with no missing
  characters and no over/underfull boxes.

## Chapter capability ledger — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, covering Chapter 6:
  the reader can count *ek, dui, tin, chār, pā̃ch* in Bengali script and say what
  **দুই** kept that Hindi *do* and Marathi *don* flattened away.
- Made `BN-C06-numbers-1-5` the chapter payoff — the chapter's only lesson, and
  its only schema-v2 one. It is typed `production`: the payoff is counting the
  five aloud, then placing *dui* in its family.
- Recorded `SPINE-COUNT-ONE-TO-FIVE` as the chapter's spine node, matching
  `BN-PATH-010` in `curriculum.json`.
- Omitted Chapters 1–5 rather than stubbing them: all 30 of their lessons are
  schema v1 and declare no `practises.knowledge`, so no payoff there could name
  atoms a lesson actually exercises. Their absence is the debt the HL05 gap
  report exists to measure.
- Measured payoff representativeness for Chapter 6 at 6/6 introduced atoms
  (1.00), comfortably above the 0.5 policy floor.

## Book warning cleanup — 2026-08-03

- Kept punctuation outside the Bengali-only font and replaced five duplicate
  recap anchors with stable chapter-qualified labels.
- Preserved Bengali in PDF bookmarks while suppressing the font-only command
  there, and mapped the vendored static font to every requested shape.
- Let short lesson pages end naturally and made the long farewell title
  breakable so the forced six-chapter build has no layout, bookmark, label,
  font, punctuation-glyph, or package warnings.

## Canonical Chapter 6 publication — 2026-08-03

- Migrated the numbers lesson to schema v2 with the shared
  `SPINE-COUNT-ONE-TO-FIVE` can-do node, a 290-second ceiling, and block-level
  knowledge closure.
- Generated the downloadable Chapter 6 from the same lesson AST and source hash
  that Language Ladder loads instead of maintaining a second content copy.
- Preserved Bengali numeral forms, the chandrabindu note, the qualified history
  of *dui*, and bookmark-safe romanization in the generated chapter; the book
  preamble now supplies the shared width-aware table renderer it uses.

## Sub-five-minute remediation — 2026-08-02

- Corrected eleven declared five-minute estimates whose computed durations were
  already between 121 and 290 seconds.
- Preserved every lesson body unchanged; no split or content reduction was
  necessary. The shared report now measures zero Bengali duration violations.
- The 290-second numbers lesson is the tightest Bengali budget and should be
  watched during later copy edits.

## Chapter 6 — Numbers 1–5, and the conservative "two"

- **Chapter 6 authored** (`BN-C06-numbers-1-5`): *ek, dui, tin, chār, pā̃ch*
  (using *ek*, not the *êk* of a first draft, which would have introduced a
  diacritic this track never defines — its established mark is **ô**).
- **দুই *dui* is the lesson.** Against Hindi *do* and Marathi *don*, Bengali
  **keeps a trace of the vowel that followed the old cluster**, which is why it
  has two syllables where its neighbours have one.
- **Two absolutes scoped back**, both of which were false as first written:
  - "No modern Indo-Aryan language kept the *dv-* cluster" is true of the
    **everyday numeral** only — *dv-* is alive in words re-borrowed straight from
    Sanskrit, like Hindi *dvār* "door."
  - "The vowel survives **only** here" ignores **Assamese, Odia and Nepali**,
    which all have *dui*. Bengali is unusual only among the four languages this
    chapter compares. (Maithili was in a first draft of that list and removed —
    it has *dū*, not *dui*.)
- **A claim removed rather than repaired.** A first draft said the numbers
  demonstrate Chapter 1's o-leaning inherent vowel. They don't — **এক** opens
  with the independent vowel এ, and none of the five contains a bare
  inherent-vowel syllable. The observation is still mentioned (it's true, and
  `BN-C01` does teach it), but now explicitly as something *not* visible in this
  data, so the learner doesn't go looking for it here.
- The **ঁ** on *pā̃ch* is named as the same **chandrabindu** the Devanagari
  tracks use.

## Chapters 2–5 — Introductions, How-are-you, Farewells, First Verbs

- Four new chapters carry Bengali from Chapter 1 to Chapter 5, matching the
  leading tracks' arc. One word per lesson, atom-first, Bengali script inline;
  every root traced (`lessons/BN-C0{2,3,4,5}-*`, `book/chapters/ch0{2,3,4,5}-*.tex`).
  Concept tags reuse the universal `HL01` taxonomy; verbs namespaced (`BN-VERB-*`).
  Two Bengali distinctives run throughout: the **zero copula** (no "is" in the
  present) and **no grammatical gender at all**.
- **Ch. 2 — Introducing Yourself**: *nām* (← *nāman* → *name*) → *āmār* (no
  gender, unlike *merā/merī*) → *āmār nām …* (the zero copula) → *tumi/āpni* (+
  *tui*: Bengali's three-way "you") → *ki* → *tomār nām ki?* → *ālāp kore bhālo
  lāglo* (*ālāp* ← Sanskrit, a rāga's opening) → practice.
- **Ch. 3 — How Are You**: *kemon* → *tumi kemon āchho?* (the verb *āchhā* — the
  copula returns for state) → *āmi* (← *asmi* → English **am**) → *bhālo* (←
  *bhadra*) → *kono bæpār nā* ("no matter" = you're welcome; *nā* ← PIE *ne) →
  practice.
- **Ch. 4 — Farewells**: *ābār* → *dækhā hôbe* (the impersonal "a seeing will
  happen") → *ābār dækhā hôbe* (the fuller form of Ch.1's *āshi*) → *kāl dækhā
  hôbe* (*kāl* = both tomorrow and yesterday ← *kāla*) → practice.
- **Ch. 5 — First Verbs**: *bôlā* → *āmi bānglā bôli* (*bôngo* → the Ganges
  delta) → *thākā* (to live ← *sthā* → English *stand/stay/state*) → *kāj kôrā*
  (to work; ← √kṛ, the root of *nômoshkar*) → practice. **The verb changes for
  person but never for gender.** Book compiles clean with XeLaTeX (0 missing
  chars, 0 undefined refs).

## Chapter 1 — Greetings (Bengali script taught inline)

- New Bengali track on the HL00 framework — Indo-Aryan, written in the Bengali
  script (vendored Noto Sans Bengali font). One word per lesson, slug ids,
  atom-first, derivations shown, LaTeX book. No reading course: the script is
  taught *inside* each word lesson.
- Chapter 1 (`lessons/BN-C01-*`):
  - **নমস্কার** nômoshkar ("hello/goodbye") — the *same* word as Sanskrit
    *namaskāra*, used to introduce Bengali's fingerprint shifts (*a→ô*, *s→sh*)
    plus the inherent-ô vowel and the স্ক conjunct.
  - **ধন্যবাদ** dhônyobad ("thank you") — Sanskrit *dhanya*+*vāda*; shows *a→ô*
    again and *v→b* (Bengali has no "v").
  - **হ্যাঁ / না** hyã / nā ("yes / no") — the *chandrabindu* nasal; *nā* on PIE
    *ne (English *no/not/none*).
  - **আচ্ছা** āchchhā ("okay / I see") — the standalone vowel-letter আ and the
    চ্ছ conjunct; the conversational workhorse.
  - **আসি** āshi ("I'll be going") — literally "I come," the "promise of return"
    goodbye shared with Tamil and Marathi; Bengali marks no gender on the verb.
  - **practice**.
- The recurring thread: Bengali's one sound-fingerprint (inherent **ô**, *s→sh*,
  *v→b*) that disguises familiar Sanskrit words, taught so the learner can
  un-shift any word back. Script facts documented in the appendix. Book compiles
  clean with XeLaTeX.
