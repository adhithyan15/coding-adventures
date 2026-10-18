## Redesign: deep one-word lessons

Major reshape after learner feedback ("I can't possibly learn *buenos días*
plus every other greeting in one go … we learn the cultural aspects and why
phrases are idiomatic, not what Spanish 101 teaches"):

- **Granularity**: the unit is now **one word or phrase per lesson**, gone
  deep — not a cluster. New `lessons/` directory, IDs `ES-C<chapter>-L<lesson>`.
- **Chapter 1 — First Words** rebuilt as 10 deep lessons (`ES-C01-L01`–`L10`):
  hola, buenos días, buenas tardes, buenas noches, gracias, por favor, adiós,
  ¿cómo estás?, me llamo, quiero. Each with: inline "Sounds you'll need",
  prerequisite cross-links, the widest honest **web of English cousins**,
  **prefix/suffix construction** taught in context (the *quaerere* →
  inquire/require/acquire/conquer/exquisite table), and a **cultural/idiomatic
  "why"** (e.g. *buenos días* as the fossil of "may God give you good days";
  *adiós* = "to God").
- **Pronunciation is no longer a chapter**: added `pronunciation-reference.md`
  (look-up reference) + inline per-lesson sound notes. The old front-loaded
  Part 0 is gone — feedback was that a sounds chapter makes people quit before
  Lesson 1.
- **HL00 spec** updated to encode all of the above (Lesson Anatomy, the
  Cousin Web + Morphology-In-Context section, "Pronunciation & Script:
  Inline, Never a Gate", new schema) and to **drop forward-references** to
  languages later in the curriculum — each track grounds on English + its own
  deep roots only.
- **Book** rebuilt: pronunciation appendix + new deep Chapter 1; new
  `cousinweb`/`culture`/`sounds`/`prefixtable` callout styles; compiles clean
  with XeLaTeX (15 pages).
- **Legacy**: the pre-redesign `units/` (old Part 0 + Part I) and old book
  chapters are superseded. Old book chapters removed (in git history); old
  `units/` kept as source material to re-excavate into deep lessons.

