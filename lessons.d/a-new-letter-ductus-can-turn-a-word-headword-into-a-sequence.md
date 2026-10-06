---
category: Testing & coverage
---

# A new letter ductus can turn a word headword into a sequence strip, which moves the real-corpus words pin in figure-targets

**Context:** `script-ductus` and `human-language-data`, adding a ductus for
seventeen Malayalam consonants, among them ന and മ.

**What happened:** the stroke-ownership pins, the filmstrip target count
(`tests/filmstrip-target-counts/malayalam.json`, 14 -> 35), every `check:*`
gate and the language-ladder suite all passed. The full human-language-data
suite then failed in `tests/figure-targets/the-real-corpus-*.case.ts`
("composes words only in scripts whose letters stand apart"), which pins the
exact list of WORD sequence strips across the corpus.

**Why:** Malayalam is in `SEPARATE_LETTER_SCRIPTS`, so a word headword whose
every letter has a cited ductus becomes a sequence strip. Three chapter-1
lessons have the headword നമ; they were waiting only for ന and മ. A fourth,
"ന മ", is a list, not a word, and is not on that pin. The target-count pin
moved with them, so it looked like the only place to update.

**Fix:** added the three lesson ids to the words pin, with a comment.

**Do differently:** before adding a letter's ductus in a script listed in
`SEPARATE_LETTER_SCRIPTS`, grep the track's writing headwords for words made
only of cited letters plus the new one. Each such word adds a strip to the
target count AND an id to the real-corpus words pin; run
`tests/figure-targets.test.ts` with the targeted set, not only the full suite.
