# Changelog

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
