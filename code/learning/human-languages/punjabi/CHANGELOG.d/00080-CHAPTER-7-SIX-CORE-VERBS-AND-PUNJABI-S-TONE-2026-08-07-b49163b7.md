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

