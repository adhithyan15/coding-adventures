# Changelog

## Punjabi chapters 1-5 regain their reading order (#12252)

- Add one global, spaced sequence to all 31 legacy lessons, recovered from the
  hand-authored book sections and closed against every prerequisite and review.
- Remove 31 missing-sequence findings plus the 31 forward prerequisite/review
  references that alphabetical filename fallback had fabricated. Punjabi's
  order-integrity backlog moves from 62 defects to zero.
- Keep the remaining learner debt visible: this metadata repair does not claim
  that Punjabi's script, writing, or exam-preparation strands are complete.

## [Unreleased]

### Added — Chapter 14, the first nine pieces of Gurmukhi (HL-C218)

Ten lessons. **Nine teach one piece each; one introduces nothing** and assembles
the greeting from pieces the reader can already write.

`scriptLessons` 0 → 10, `taughtGlyphs` 0 → 9, `neverTaughtGlyphs` **45 → 36**.

**The fourth idea is different here, and that is the finding.** Gujarati and
Marathi could teach inherent vowel → mātrā → virama → conjunct because their
greetings contain a conjunct. **Punjabi's does not.** ਨਮਸਤੇ has four plain
consonants in a row, so a virama lesson would have taught machinery with nothing
to spend it on.

What Gurmukhi needs instead is the **bindi** — a dot above the head-line that
nasalises a vowel — because both *yes* and *no* require it. So the fourth idea is
nasalisation, and the chapter ends by **naming the conjunct as still to come**
rather than claiming the system is complete.

The script's own history is taught on the first shape: **Gurmukhi means "from the
mouth of the Guru"**, standardised in one go in the sixteenth century by Guru
Angad — which is why it is tidier than the scripts around it. It keeps the
head-line that Gujarati erases.


## Chapters 10–13 — the pre-A1 noun tranche (HL-C41 continuation) — 2026-08-08

Fourteen everyday-noun lessons across four new chapters (10–13), continuing the
cross-track pre-A1 vocabulary program and confirming the same measured
mechanism a further time: `vocabularyOf()` counts distinct `headword:` strings
1:1 with lessons, so fourteen new word lessons move Punjabi's pre-A1
vocabulary by exactly fourteen (22 → 36 distinct headwords at or below
pre-A1). All seven pre-A1 spine nodes are now realized.

- **Ch. 10 — Water, Tea, Milk, and Bread** (`SPINE-POLITE-REQUEST-REPAIR`,
  previously unrealized — this is the resolution): ਪਾਣੀ, ਚਾਹ, ਦੁੱਧ, ਰੋਟੀ. Sorts
  the four by origin: ਪਾਣੀ and ਦੁੱਧ inherited with a known root, ਰੋਟੀ inherited
  with an unknown one, ਚਾਹ loaned from Chinese by the overland route and
  carrying Punjabi's first noun-level tone.
- **Ch. 11 — Friend, Family, Brother, and Sister** (`SPINE-EXCHANGE-NAMES`):
  ਦੋਸਤ, ਪਰਿਵਾਰ, ਭਰਾ, ਭੈਣ. States that ਭਰਾ is the unbroken PIE cousin of
  *brother* while ਭੈਣ, from *bhaginī*, is **not** a cousin of *sister* — the
  same initial-bh-to-low-tone shift opens both, but only one lineage holds.
- **Ch. 12 — Eye, Ear, Mouth, and Nose** (`SPINE-CHECK-WELLBEING`): ਅੱਖ, ਕੰਨ,
  ਮੂੰਹ, ਨੱਕ. ਅੱਖ and ਨੱਕ are unbroken PIE cousins of *eye* and *nose* that kept
  their doubled consonants rather than trading a sound for a tone; ਕੰਨ is a
  false friend of ਕਰਨਾ ("to do"), matching in spelling but not in root.
- **Ch. 13 — Heart and Head**: ਦਿਲ, ਸਿਰ. ਦਿਲ is Punjabi's one borrowed body
  word against four inherited ones; ਸਿਰ traces to the same root as English
  *horn*, not *head*, and its resemblance to Persian *sar* is convergence from
  a shared Proto-Indo-Iranian ancestor rather than a loan — the opposite case
  from ਦਿਲ, genuinely borrowed from Persian despite sharing the same deep
  Indo-European root as English *heart*.
- Book compiles clean with XeLaTeX; all four new chapters wired into
  `book-generation.json`.

## Chapters 8 and 9 — the eight verbs, and the other half of the tone rule — 2026-08-07

- Authored **eight** schema-v2 lessons realizing the eight `SPINE-SAY-WHAT-I-DO`
  concepts fifteen other tracks already teach: `PA-C08-sochna` (`VERB-THINK`),
  `PA-C08-samajhna` (`VERB-UNDERSTAND`), `PA-C08-parhna` (`VERB-READ`),
  `PA-C08-likhna` (`VERB-WRITE`), `PA-C09-laina` (`VERB-TAKE`),
  `PA-C09-puchhna` (`VERB-ASK`), `PA-C09-madad-karna` (`VERB-HELP`), and
  `PA-C09-pasand` (`VERB-LIKE-LOVE`).
- Split them into **two chapters of four**, never one of eight. Each chapter
  introduces exactly 10 atoms against the `maxNewAtomsPerChapter` budget of 12
  that a single chapter of eight would have blown, and no lesson introduces more
  than 3. Chapter 7's own 20-atom overrun is untouched and still reported.
- Paid the **other half of the tone rule**. Chapter 7's `PA-C07-khana` taught
  the word-initial case — **ਘ** deaspirated and left a **low, rising** pitch, so
  **ਘੋੜਾ** *kòṛā* "horse" and **ਕੋੜਾ** *koṛā* "whip" differ by tune alone — and
  promised the mirror without giving it. `PA-C08-samajhna` gives it: **ਝ** is not
  word-initial in *samajhṇā*, so the pitch on the vowel **before** it goes
  **high and falls** (*samájṇā*), and `PA-C08-parhna` runs the same rule on
  **ੜ੍ਹ** (*páṛnā*). Better, the **ਝ** *is* the old breathy *dh* of *budh-*, so
  the falling pitch is that root's last trace.
- Gave each lesson one idea. *sochṇā* ← *śocyate* on *śuc-* "burn, glow;
  grieve" — hence the noun *soch* meaning both a thought and a worry, and the
  name **Aśoka**, "sorrowless." *samajhṇā* ← *sambudhyate* on *budh-* "to wake,"
  the root that named the **Buddha**, PIE \**bʰewdʰ-* → English *bode*,
  *forebode*, *forbid*. *paṛhnā* ← *paṭhati* "recites, reads aloud" — reading
  named for the mouth where Chapter 7's *vekhṇā* was named for the eye, and the
  same root gives **ਪਾਠ** *pāṭh*. *likhṇā* ← *likhati* "scratches," the picture
  Latin *scrībere* and Old English *wrītan* reached independently. *laiṇā* ←
  *labhate* → *lahaï* → *lai*, a breathy *bh* worn away to nothing. *puchhṇā* ←
  *pṛcchati* ← \**preḱ-* → *pray*, *precarious*, *postulate*, German *fragen*.
  *madad* is Arabic on the root *m-d-d* "to stretch out," through Persian.
  *pasand* is Persian, *\*pati-* "towards" on *\*sand* "to look good."
- Introduced the **subjoined ਹ** (**ੜ੍ਹ**) as the track's first Gurmukhi mark
  that hangs *below* a letter rather than from the top line.
- Said "we do not know" four times rather than padding. *śuc-*, *paṭh-*,
  *lebʰ-* and *likh-* have no secure English cousin (and *paṭh-*'s own origin is
  unsettled), and each lesson states the limit instead of inventing one. The tie
  from *pasand*'s *\*sand* to Latin *candēre* and Sanskrit *candra* is offered
  as **proposed, not settled** — the same treatment the Urdu track gave it.
- Made the two-vocabularies thread **testable**, not asserted. Chapter 6 argued
  that Punjabi *panj* and Persian *panj* match exactly and are still not a loan.
  `PA-C09-puchhna` supplies the mirror — *puchhṇā* and Persian *porsīdan* match
  not at all and *are* cousins — and `PA-C09-madad-karna` supplies the opposite
  answer to the same question: *madad* really is borrowed, with no Sanskrit path
  at all.
- Reinforced at **two cadences**. Every lesson's `practises.knowledge` names
  atoms from the immediately preceding one to three lessons, across the chapter
  seam, and each payoff reaches several chapters back. Punjabi's never-revisited
  atoms fall from **21 of 31 to 3 of 51**; the three that remain are the three
  introduced by the final lesson of the track, which no later lesson exists to
  retrieve. Corpus-wide the figure moves from 668 of 1753 (38%) to 650 of 1773
  (37%).
- Used the canonical `## The letters in this word` heading, which classifies as
  a `script` block and is DETACHABLE. All eight lessons derive as `sight` with a
  **voice core**, so the track's rescued count rises from 21 to 29, chapter-prefix
  reachability from 38 lessons to 46, and drivability from 97% to 98%.
- Wiring: `PA-PATH-012`/`PA-PATH-013` and `PA-EXT-012-MIND-VERBS`/
  `PA-EXT-013-DOING-VERBS` in `curriculum.json`, with all eight concepts dropped
  from the `SPINE-SAY-WHAT-I-DO` omission ledger (36 omits down to 28); both
  chapters registered in `core/book-generation.json`, `chapters.json` and
  `book/book.tex`.
- Both payoffs assess **every** atom their own chapter introduces — 10 of 10 and
  10 of 10, representativeness 1.00 against the 0.5 floor.
- Verified: the nine-chapter book compiles under XeLaTeX to 56 pages with **zero**
  `Missing character` warnings. Perso-Arabic script was deliberately kept out of
  the lesson bodies — the Punjabi book loads no Perso-Arabic font, and a draft
  that spelled *porsīdan* in Persian script produced six missing glyphs.

## Chapter 7 — six core verbs, and Punjabi's tone — 2026-08-07

- Authored six schema-v2 lessons realizing the shared `SPINE-SAY-WHAT-I-DO`
  node with the **canonical** concept tags the spine owns, not namespaced ones:
  `PA-C07-hona` (`VERB-BE`), `PA-C07-jana` (`VERB-GO`), `PA-C07-auna`
  (`VERB-COME`), `PA-C07-khana` (`VERB-EAT`), `PA-C07-vekhna` (`VERB-SEE`), and
  `PA-C07-janna` (`VERB-KNOW`). Before this, the track had four verb lessons and
  every one of them was namespaced (`PA-VERB-BOLNA`, `PA-VERB-RAHNA`, …), so
  Punjabi realized none of the spine's verb concepts.
- Gave each lesson exactly one idea. *hoṇā*: "to be" is two ancient verbs
  braided — the infinitive from Sanskrit *bhavati* / *bhū-* (English *be*), the
  present *hai* from *asti* / *as-* (English *is*) — the same seam English keeps
  between *am/is/are* and *be/been*. *jāṇā*: every Sanskrit initial *y-* became a
  Punjabi *j-*, a rule the reader can run backwards (*yoga* → *jog*, *yuvan* →
  *javān*, *yamunā* → *Jamnā*). *āuṇā*: "come" is *ā-* "toward" on *gam-* "go,"
  and *gam-* is PIE \**gʷem-*, the root English inherited as *come*. *vekhṇā*:
  the verb is built on *akṣi*, the **eye** — PIE \**h₃ekʷ-*, behind English
  *eye*, *window*, *ocular*, *optic*, *autopsy*. *jāṇnā*: one held *n* from
  *jāṇā*, from *jñā-* ← \**ǵneh₃-* → *know*, *can*, *cunning*, *notice*,
  *diagnosis*.
- Gave **tone** its own lesson, on *khāṇā*. The old breathy ਘ ਝ ਢ ਧ ਭ stopped
  being breathy and left a pitch on the vowel, so **ਘੋੜਾ** *kòṛā* "horse" and
  **ਕੋੜਾ** *koṛā* "whip" differ by tune alone. This is the track's signature
  fact — Punjabi is the one major Indo-Aryan language with tone — and
  `pronunciation-reference.md` had promised the lessons would flag it where it
  changes a word. It is now paid. *Khāṇā*'s own ਖ is voiceless and stays flat,
  which is why the verb is a safe place to meet the contrast.
- Marked the thin cousin webs as thin rather than padding them: *yā-* left
  English nothing usable, and no English cousin has been securely traced from
  *khād-*. Both lessons say so.
- Kept every lesson under five minutes and **drivable**. Measured effective
  durations are 195–270 s (3.3–4.5 min); all six derive as `voice`, so
  Chapter 7 is the track's second fully drivable chapter and its longest.
- Added `PA-PATH-011` to `curriculum.json` under `SPINE-SAY-WHAT-I-DO`, and
  dropped `VERB-BE`, `VERB-GO`, `VERB-COME`, `VERB-EAT`, `VERB-SEE`, and
  `VERB-KNOW` from that node's `omits` ledger — without which the six lessons
  would have been orphaned realizations.
- Added the Chapter 7 entry to `chapters.json`, with `PA-C07-janna` as the
  production payoff (11 of 11 atoms the chapter's lessons carry into it).
- Registered the chapter in `core/book-generation.json` and `book/book.tex`, and
  generated `book/chapters/ch07-core-verbs.tex` from the lessons. A full
  seven-chapter XeLaTeX build is clean: zero missing characters and zero
  warnings of every scanned class. Syllable dots are kept out of Gurmukhi spans.

## Chapter capability ledger — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, covering Chapter 6:
  the reader can count *ikk, do, tinn, chār, panj* in Gurmukhi, tell the addak
  from the tippi, and argue **ਪੰਜ**'s origin.
- Made `PA-C06-panj-convergence` the chapter payoff — the chapter's last
  schema-v2 lesson by sequence (350). Its task is the convergence argument:
  Sanskrit *pañca* voiced after a nasal to *-nj-*, evidenced by *panjāh* against
  Hindi *pacās*, while Iranian *panč* reached the same shape independently.
- Recorded `SPINE-COUNT-ONE-TO-FIVE` as the chapter's spine node, matching
  `PA-PATH-010` in `curriculum.json`.
- Omitted Chapters 1–5 rather than stubbing them: all 31 of their lessons are
  schema v1 and declare no `practises.knowledge`, so no payoff there could name
  atoms a lesson actually exercises. Their absence is the debt the HL05 gap
  report exists to measure.
- Measured payoff representativeness for Chapter 6 at 8/11 introduced atoms
  (0.73). The three outside the payoff are the Gurmukhi script atoms, which the
  convergence lesson does not re-exercise; they were not padded in.

## Warning-clean six-chapter book — 2026-08-03

- Gave each handwritten recap its stable canonical lesson label.
- Kept Gurmukhi text in PDF bookmarks while removing font-only wrappers from
  Hyperref's string conversion.
- Selected the vendored static Gurmukhi font for every requested text shape,
  let short callout pages end naturally, and shortened the two-forms-of-you
  running title.
- The forced 30-page build now has no missing glyphs, duplicate labels,
  overfull or underfull boxes, font warnings, Hyperref warnings, or other LaTeX
  package warnings.

## Canonical Chapter 6 publication — 2026-08-03

- Migrated both number lessons to schema v2 with the shared
  `SPINE-COUNT-ONE-TO-FIVE` can-do node, explicit sub-five-minute budgets, and
  block-level knowledge closure.
- Generated the downloadable Chapter 6 from the same ordered lesson AST and
  source hash that Language Ladder loads, rather than maintaining another copy.
- Preserved Gurmukhi inline with the book's vendored font and used romanized
  section short titles for stable PDF bookmarks.

## Sub-five-minute remediation — 2026-08-02

- Corrected nine declared five-minute estimates whose computed durations were
  already between 106 and 172 seconds.
- Split the genuinely long numbers lesson into a 229-second counting/script
  lesson and a prerequisite-ordered 241-second etymology lesson.
- Preserved the addak/tippi distinction, Chapter 5 *panjābī* callback,
  *panjāh/pacās* evidence, and Punjabi/Persian convergence explanation. The
  shared report now measures zero Punjabi duration violations.
- Updated the roadmap and session map to expose both Chapter 6 lesson boundaries.
  Chapter 6's missing one-source book publication remains explicit in the shared
  backlog.

## Chapter 6 — Numbers 1–5, and *panj* arriving as a number

- **Chapter 6 authored** (`PA-C06-numbers-1-5`): *ikk, do, tinn, chār, panj*, in
  **Gurmukhi**. The lesson distinguishes two marks that look alike and do
  different jobs: **ੱ** *addak* **doubles the following consonant** (*ikk*),
  **ੰ** *tippi* marks a **nasal** (*tinn*, *panj*). An earlier draft credited the
  addak with *tinn*'s doubling — that word is spelled with the **tippi**, and the
  doubling is in the pronunciation.
- **Built as a payoff to Chapter 5, not a re-reveal.** `PA-C05-main-punjabi-
  bolda-han` already takes *panjābī* apart as Persian *panj* "five" + *āb*
  "water," and already names the English *five* / Latin *quīnque* / Greek *pente*
  cousins. An earlier draft of this lesson presented all of that as new — caught
  by grepping the track before authoring. It now says plainly that the learner
  has known *panj* since Ch. 5 **as a piece of a place-name**, and that what
  changes here is that it becomes **an ordinary number you count with**. The
  lesson lists Ch. 5 in both `prerequisites` and `reviews_of`.
- **The genuinely new observation, and the one the first draft got wrong.**
  Punjabi's five ends in **-j** where every Indic neighbour ends in **-ch**
  (*pañca, pāṁch, pā̃ch, pā̃ch*), which looks like the Persian *panj* — and the
  draft therefore called it "the Iranian branch showing beside the Indic ones,"
  implying Punjabi's numeral was the Persian word. **It isn't.** Punjabi's *panj*
  is its own inherited descendant of *pañca*, by the regular north-western
  Indo-Aryan **voicing of a stop after a nasal** — the same change that gives
  *panjāh* "fifty" (← *pañcāśat*) — set against **Hindi *pacās***, which has no
  *j* at all, since a same-source contrast is what actually demonstrates the rule.
  (A first draft also cited *pandrāṁ* "fifteen"; that *-nd-* comes from *daśa*
  "ten", not from voicing *pañca*'s *c*, and Hindi has *pandrah* with the same
  cluster — so it proved nothing and was dropped.) Persian voiced its own *č*
  separately, on the way from Old to Middle Persian.
  - So the match is **convergence, not borrowing** — two branches of
    Indo-Iranian arriving at the same shape independently. Which makes a better
    payoff than the original: it is *why* the Persian place-name *Punjab* sits so
    comfortably in Punjabi that you'd never guess it was foreign. The borrowed
    word and the native word had already grown into the same shape.
  - The **place-name** is still Persian, for the administrative-language reason.
    It's the **numeral** that's homegrown, and the lesson now says exactly that.

## Chapters 2–5 — Introductions, How-are-you, Farewells, First Verbs

- Four new chapters carry Punjabi from Chapter 1 to Chapter 5, matching the
  leading tracks' arc. One word per lesson, atom-first, Gurmukhi inline; every
  root traced (`lessons/PA-C0{2,3,4,5}-*`, `book/chapters/ch0{2,3,4,5}-*.tex`).
  Concept tags reuse the universal `HL01` taxonomy; verbs namespaced (`PA-VERB-*`).
  Punjabi's two-vocabularies thread (Sanskritic vs. Perso-Arabic) runs throughout.
- **Ch. 2 — Introducing Yourself**: *nāṁ* (← *nāman* → *name*) → *merā* → *hai*
  (← *asti* → *is*) → *merā nāṁ … hai* → *tū̃/tusī̃* (← *\*tū* → *thou*) → *kī*
  (← *ka-* → *what*) → *tuhāḍā nāṁ kī hai?* → *khushī* (Persian — the second
  vocabulary) → practice. SOV order; two-level "you".
- **Ch. 3 — How Are You**: *kivēṁ* (how) → *tusī̃ kivēṁ ho?* → *maiṁ* (I; the
  *hāṁ* "am"/"yes" homophone) → *ṭhīk* (native "fine") → *koī gall nahīṁ*
  (you're welcome = "no matter"; *nahīṁ* ← PIE *ne) → practice.
- **Ch. 4 — Farewells**: *phir* → *milāṁge* → *phir milāṁge* (vs. Ch.1's formal
  *sat srī akāl*) → *rabb rākhā* ("God keep you"; **Arabic** *rabb* + **Sanskrit**
  *rakṣ-* — the two vocabularies in one blessing) → practice.
- **Ch. 5 — First Verbs**: *bolṇā* (the *-ṇā* infinitive) → *maiṁ panjābī boldā
  hāṁ* (I speak Punjabi; *panj* "five," cousin of English *five*, + *āb* "river"
  = "the five-rivers language") → *rahiṇā* (to live) → *kamm karnā* (to work; ←
  √kṛ, the root of *namaskār*) → practice. The gendered present habitual. Book
  compiles clean with XeLaTeX (0 missing chars, 0 undefined refs).

## Chapter 1 — Greetings (Gurmukhi taught inline)

- New Punjabi track on the HL00 framework — Indo-Aryan, written in Gurmukhi
  (vendored Noto Sans Gurmukhi font). One word per lesson, slug ids, atom-first,
  derivations shown, LaTeX book. No reading course: the script is taught *inside*
  each word lesson.
- Chapter 1 (`lessons/PA-C01-*`):
  - **ਸਤਿ ਸ੍ਰੀ ਅਕਾਲ** sat srī akāl ("hello/goodbye," the Sikh greeting) — taught
    as a creed: *sat* "truth" (← Sanskrit *satya*, English *sooth*) + *srī*
    "revered" (← *śrī*) + *akāl* "timeless" (*a-* "not" + *kāl* "time," a name of
    God). Introduces the Gurmukhi inherent-a, vowel signs, and vowel carriers.
  - **ਨਮਸਤੇ** namaste ("hello," general/pan-Indian) — Sanskrit *namas* + *te*,
    the same word as Hindi; contrasted with the Sikh *sat srī akāl*.
  - **ਧੰਨਵਾਦ** dhannavād ("thank you," Sanskritic) — Sanskrit *dhanya* + *vāda*;
    the *ṭippi* nasal.
  - **ਸ਼ੁਕਰੀਆ** shukrīā ("thank you," Perso-Arabic) — from Arabic *shukr* via
    Persian; the *pair bindi* (ਸ਼) that marks borrowed sounds. The two-vocabulary
    thread.
  - **ਹਾਂ / ਨਹੀਂ** hāṇ / nahīṇ ("yes / no") — the *bindi* nasal; *nahīṇ* on PIE
    *ne (English *no/not/none*).
  - **practice**.
- The recurring thread: Punjab's **two vocabularies** (Sanskritic *dhannavād* vs.
  Perso-Arabic *shukrīā*), the Gurmukhi script's own annotation of loanwords
  (pair bindi), and the Sikh greeting as a small creed. Gurmukhi facts and
  Punjabi's tone system documented in the appendix. Book compiles clean with
  XeLaTeX.
