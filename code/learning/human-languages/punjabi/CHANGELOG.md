# Changelog

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
