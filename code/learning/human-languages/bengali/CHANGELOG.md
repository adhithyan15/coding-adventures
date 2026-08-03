# Changelog

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
