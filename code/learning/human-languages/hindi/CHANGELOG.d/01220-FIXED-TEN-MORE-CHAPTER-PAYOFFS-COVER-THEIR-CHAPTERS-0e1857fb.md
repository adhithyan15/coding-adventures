## Fixed — ten more chapter payoffs cover their chapters

Twelve chapter payoffs were below the 0.5 `chapter-payoff-not-representative`
floor. Each payoff lesson now recalls the sibling atoms its chapter introduces
before it, with prompts grounded only in what the introducing lessons taught.
"Before" means earlier on both orders the validators enforce: lesson
`sequence` and the curriculum path. Atoms from script segments that come later
by sequence are not assessed. Each chapter's `payoff.note` names those segments
and says why. Ten chapters now clear the floor. Chapters 8 and 14 cannot
without reordering lessons, so they stay below it with notes that explain why.

| chapter | payoff | before | after |
|---|---|---|---|
| 6 | HI-C06-paanch-nasal | 1/6 | 4/6 |
| 7 | HI-C07-nahin | 3/7 | 5/7 |
| 8 | HI-C08-kripaya | 2/5 | 2/5 (note only) |
| 10 | HI-C10-shanivaar-ravivaar | 2/6 | 4/6 |
| 11 | HI-C11-laal-niila | 2/6 | 4/6 |
| 12 | HI-C12-bhaai-bahin | 2/6 | 4/6 |
| 13 | HI-C13-haath | 2/5 | 3/5 |
| 14 | HI-C14-ritu | 2/7 | 3/7 |
| 15 | HI-C15-paani-roti | 2/5 | 3/5 |
| 19 | HI-C19-age-grammar | 1/3 | 2/3 |
| 22 | HI-C22-gyarah-bees | 3/8 | 8/8 |
| 32 | HI-C32-evening-register | 1/5 | 5/5 |

- Chapters 6, 7, 10, 11, 12, 13 and 19 each gain one "Guided Practice — …"
  section in the payoff lesson. It recalls the chapter's earlier speaking
  lesson: the five numbers and the *tīn*/*chār* weight trade; *hāṃ* and *jī
  hāṃ*; the deity + *vār* pattern and its Babylonian origin; *kāla* and
  *safed*; *pitā*/*mātā* and *bāp*/*māṁ*; *sir* < *śiras*; *umr* beside
  *āyu*.
- Chapters 14 and 15 also recall the anchor words *ghī* and *ḍhol*, so
  HI-C14-ghee and HI-C15-dhol join the payoffs' prerequisites.
- Chapter 22's warm-up now asks for *os* and *ūn*. Its warm-up marker already
  credited HI-LEX-ANCHOR-OS, but the warm-up never asked for it. A new "Script — the
  chapter's three vowels" section points to उ, ऊ and ओ. The teen and उन्नीस
  prose is tightened to make room. HI-S129-letter-o joins the prerequisites.
- Chapter 32 gains a recall of संध्या and साँझ and a "Script — two letters
  from this chapter" section that finds श and झ. HI-S133-letter-jha and
  HI-S145-letter-sha join the prerequisites.
- Chapter 8: HI-C08-kripaya is first in the chapter by sequence and last on
  the path. Its three script segments are the other way round on both
  orders. Moving the payoff to HI-S144-vowel-sign-vocalic-r was tried and
  fails `curriculum-prerequisite-order`.
- Chapter 14: HI-S125-letter-vocalic-r is last by sequence but sits before
  HI-C14-ritu on the path. So it cannot assess the six-season atoms that the
  chapter's can-do names.

Every added atom is listed in `practises.knowledge`, and in
`requires.knowledge` where that list carries chapter atoms. Recall prompts
use romanization for words whose letters come later, so script closure stays
at zero. `duration.max_seconds` now covers the computed duration. All are
under 300 s: paanch-nasal 200, nahin 280, shanivaar-ravivaar 230, laal-niila
270, bhaai-bahin 280, haath 220, ritu 280, paani-roti 250, age-grammar 240,
gyarah-bees 290 and evening-register 260.
