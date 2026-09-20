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

