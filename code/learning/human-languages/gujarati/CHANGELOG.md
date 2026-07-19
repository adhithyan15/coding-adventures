# Changelog

## Chapters 1–5 — new Gujarati track (Gujarati script taught inline)

New Gujarati track on the HL00 framework — Indo-Aryan, written in the Gujarati
script (vendored Noto Sans Gujarati font, `data/scripts/gujarati.json`). One
word/phrase per lesson, slug ids, atom-first assembly, every atom traced to its
root, a publishable LaTeX book. No reading course: the script — the "headless"
Devanagari-without-the-top-line — is taught *inside* each word lesson.

- **Chapter 1 — Greetings** (`lessons/GU-C01-*`): namaste, ābhār (Sanskrit
  *bhṛ* "to bear," cousin of English *bear*, Portuguese *obrigado*), hā/nā,
  sārũ (introduces the **three genders** *sāro/sārī/sārũ*), āvjo ("come again"),
  practice. Foregrounds the two Gujarati distinctives from the first page: the
  **missing top line** and the **three genders**.
- **Chapter 2 — Introducing Yourself** (`lessons/GU-C02-*`): nām (PIE
  *h₃nómn̥*, English *name*), mārũ (gender agreement again), **chhe** (Gujarati's
  own copula, not Hindi *hai*), "mārũ nām … chhe", tũ/tame (courtesy-by-plural),
  shũ ("what," the odd *sh-* cousin in a *k-* family), "tamārũ nām shũ chhe?",
  ānand ("joy"), practice.
- **Chapter 3 — How Are You** (`lessons/GU-C03-*`): kem (the *k-* questions,
  PIE *kʷo-*), "tame kem chho?", hũ (*aham* → Latin *ego*, English *I*), **majā**
  (Persian *maza* — the Perso-Arabic trade layer), vāndho nahī, practice.
- **Chapter 4 — Farewells** (`lessons/GU-C04-*`): pāchhā, maḷīshũ (the future
  is an ending; the retroflex **ḷ** ળ Gujarati keeps), "pāchhā maḷīshũ", kāle
  (*kāl* = both "tomorrow" and "yesterday"), practice.
- **Chapter 5 — The First Verbs** (`lessons/GU-C05-*`): bolvũ (the *-vũ*
  infinitive; stem + person + copula), "hũ gujarātī bolũ chhũ" (*gujarātī* ←
  the **Gurjar** people), rahevũ (postposition *-mā*), kām karvũ (Sanskrit *kṛ*
  — root of *namaskār*, *karma*), practice.

Infrastructure: vendored `_fonts/NotoSansGujarati-Static.ttf` (shaping verified);
`data/scripts/gujarati.json` (29 letters, 10 marks, abugida) per the HL01 schema;
`book/preamble.tex` with the `\gu{}` command and IAST `newunicodechar` maps.
Book compiles clean with XeLaTeX (0 missing characters, 0 undefined references)
and was rasterized and visually QA'd. Note for this script: Noto Sans Gujarati
carries no Latin punctuation glyphs, so all `.?!-` are kept **outside** the
`\gu{}` spans (they tofu inside).
