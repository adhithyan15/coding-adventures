## Fixed — chapters 2 to 5 payoffs cover their chapters

The payoffs of chapters 2 to 5 assessed only the headline words and patterns
of each chapter, not the etymology, grammar and script atoms the chapter's
lessons also introduce, so each was below the 0.5
`chapter-payoff-not-representative` floor: SA-C02-practice 9/23 (0.39),
SA-C03-practice 7/17 (0.41), SA-C04-practice 6/13 (0.46) and SA-C05-practice
7/15 (0.47). Each practice lesson now exercises the missing atoms, with every
prompt and answer taken from the lesson that introduced it, and
`chapters.d/0002.json` to `0005.json` list them: **23/23, 16/17, 13/13 and
14/15**.

- **New spoken recall.** Each lesson gains a `## Guided Practice — …` section
  of `[YOU SAY: …]` prompts with their answers: chapter 2 the two words for
  "you", the respectful name question and the roots of *nāma*, *mama*, *asti*,
  *kim* and *ānandaḥ*; chapter 3 the roots of *katham*, *aham*, *kuśalam* and
  *cintā*, pronoun dropping and the *-mi* ending; chapter 4 the literal
  *punar-darśanāya*, the dative *-āya*, *darśana* as beholding and the root of
  *śvaḥ*; chapter 5 the roots *vad*, *vas* and *kṛ*, *saṁskṛta* and sandhi.
- **New script sections.** Each lesson gains a `## Script — …` section showing
  the chapter's shapes out of order with a `[YOU POINT: …]` cue and a spoken
  answer: आ भ ◌ि ◌ः (chapter 2), म ◌ा (3), र प (4), य ◌ृ (5). Skills add
  `reading`.
- **Existing sections tagged honestly.** Prompts already asked in "How to
  answer" now carry their atoms: the copula trio in chapter 3, *punar-* and
  the one-word-one-day contrast in chapter 4.
- **Prerequisites.** Each lesson adds the script lesson that ends the
  chapter's script chain before it: SA-S109-vowel-sign-i (141),
  SA-S01-letter-ma (181), SA-S113-letter-pa (241) and
  SA-S205-vowel-sign-vocalic-r (281). SA-C02-practice also lists
  SA-C02-bhavan and SA-C02-tvam in `reviews_of`.
- **Not assessed.** ◌ु (SA-SCRIPT-RECOG-203, sequence 201) and अ
  (SA-SCRIPT-RECOG-03, sequence 301) are introduced just after the chapter 3
  and chapter 5 checkpoints, so those payoffs leave them out and say so in
  their notes.
- **Durations.** Computed 250 / 207 / 173 / 187 s; SA-C02-practice's
  `max_seconds` rises from 240 to 250, the others stay at 240.
