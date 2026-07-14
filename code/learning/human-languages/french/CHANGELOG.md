# Changelog

## Beginner-audience pass — Spanish no longer assumed as prior knowledge

Corrected a systemic violation of HL00's Audience rule: the book and practice
lessons addressed a reader who was "also learning Spanish" and leaned on
Spanish as knowledge already owned. The books are for a true beginner whose
only shared language is English; Spanish comparisons are enrichment the text
must supply in full, not a baseline it may assume.

- Preface rewritten: drops "Because the reader is also learning Spanish…" and
  "exactly as in the Spanish book"; states the true-beginner framing and that
  every Spanish comparison is supplied by the text (a reader who knows Spanish
  "simply nods along").
- Chapter 1 (`book/chapters/ch01-greetings.tex`) and the matching practice
  lessons: recast every "Spanish twin," "the *bueno/buena* machine from
  Spanish," "One mercy over Spanish," and "you know this from Spanish" into
  self-contained "Spanish, another daughter of Latin, does X" enrichment.
  Section title "*bien* — and a Spanish twin" → "*bien* — 'well'."
- Filled the two missing noun plurals the standard wants: *les soirs*,
  *les nuits* (a new Grammar Lens on *soir*, extended on *nuit*).
- Book still compiles clean with XeLaTeX (13 pages).

## Chapter 1 — Greetings (track bootstrapped)

- New French track, built on the same HL00 framework as Spanish: one word per
  lesson, slug ids, gender-before-nouns, atom-first assembly, derivations
  shown (not just roots named), LaTeX book.
- Chapter 1 (`lessons/FR-C01-*`), atom-first:
  - **salut** (informal hi ← Latin *salus* "health") · **bien** ("well" ←
    *bene*; the Spanish twin) · **bon / bonne** ("good" ← *bonus*; agreement)
  - **le / la / les** ("the"; grammatical gender ← Latin *ille/illa/illos*,
    same as Spanish *el/la*, also the source of *il/elle*)
  - **jour** ("day" ← *diurnum* ← *dies*; the detour that gives English
    *journal*/*journey* and explains why French *jour* ≠ Spanish *día*)
  - **bonjour** (assembled; *singular*, contrasted with plural *buenos días*)
  - **soir** ("evening" ← *sērus* "late"; parallels Spanish *tarde* ←
    *tardus*) · **bonsoir**
  - **nuit** ("night" ← *noctem*; the *-ct-→-ch-* (Spanish) vs *-ct-→-it-*
    (French) sound-change table) · **bonne nuit** (feminine agreement)
  - **practice**
- Grounds each word against English **and Spanish** (the learner's in-progress
  language), foregrounding the Romance twins' differences.
- Book compiles clean with XeLaTeX (13 pages); the CI workflow auto-discovers
  `french/book/` and builds it as a PDF artifact.
