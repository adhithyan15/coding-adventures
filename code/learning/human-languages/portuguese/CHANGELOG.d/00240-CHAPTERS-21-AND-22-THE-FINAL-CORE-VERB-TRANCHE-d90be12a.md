## Chapters 21 and 22 — the final core-verb tranche

- Added seven lessons realising the **last seven core-verb concepts no track in
  the corpus taught**, verbatim. **Chapter 21, "Bringing, Getting, Waiting,
  Buying"**: `VERB-BRING` (`PT-C21-trazer`), `VERB-GET` (`PT-C21-conseguir`),
  `VERB-WAIT` (`PT-C21-esperar`), `VERB-BUY` (`PT-C21-comprar`). **Chapter 22,
  "Answering, Meeting, and Three Ways to Play"**: `VERB-ANSWER`
  (`PT-C22-responder`), `VERB-MEET` (`PT-C22-encontrar`), `VERB-PLAY`
  (`PT-C22-jogar-brincar-tocar`). Portuguese moves from **15 to 22 of the core
  40**, and the corpus-wide universally-missing list goes to **zero**: every one
  of the 40 shared verb concepts is now realised somewhere.
- **The signature is that Portuguese splits English "play" three ways.**
  *Jogar* is a game with rules and a winner; *brincar* is a child at play, and
  from there joking; *tocar* is an instrument. No other split in this tranche is
  as wide, and *brincar* is the one English genuinely has no word for — "play"
  is too broad and "mess about" is dismissive. That lesson is Chapter 22's
  payoff.
- **Corrected before authoring: *comprar* is not related to English *compare*.**
  The plausible-looking link is a homograph trap. Latin had **two** verbs spelled
  *comparāre*: one from *com-* + *parāre* ("to make ready, to procure"), which is
  the ancestor of *comprar*, and one from *com-* + *pār* ("equal, to match"),
  which is the ancestor of English *compare*. The lesson teaches the real family
  — *prepare*, *repair*, *separate*, *apparatus*, *emperor* — and names the trap
  explicitly rather than quietly avoiding it.
- **Two more roots are flagged as unsettled rather than guessed.** *Brincar*'s
  leading account runs through *brinco* ("earring") from Latin *vinculum* ("a
  bond"), with a Germanic "gleam" word as a live rival; neither is settled, and
  the lesson says so. *Tocar* ← Vulgar Latin \**toccāre* ("to knock, to strike")
  is itself most likely imitative — which is stated as such. This follows the
  precedent `PT-C20-tomar-pegar` set for *tomar*.
- **Three verbs answer more English words than a gloss suggests.** *Conseguir*
  with a following infinitive means "to manage to, to succeed in" (*consigo
  ajudar*), which is far more frequent than the dictionary "to get". *Esperar*
  covers *wait*, *hope* and *expect* in one verb. *Tocar* keeps "to touch"
  alongside "to play".
- **Meeting is split with a verb the track already taught.** Chapter 18 taught
  both *saber* and *conhecer*, so Chapter 22 builds the contrast rather than
  re-teaching it: *encontrar* meets someone you already know; *conhecer* meets
  someone for the first time.
- **Reinforcement.** Every lesson practises atoms from the one to three lessons
  immediately before it, across the chapter seam, and each payoff reaches back
  several chapters. Measured effect on the track: atoms never revisited at any
  distance fall from **22 to 3**, and Portuguese's count of atoms that miss a
  reinforcement window they were long enough to have falls to **zero**. The
  remaining 3 are the final lesson's own atoms, which no later lesson exists to
  practise. Rescued from permanent orphanhood: `PT-ETYMON-TOMAR-PEGAR-03`,
  `PT-ETYMON-AJUDAR-03`, `PT-ETYMON-ESCREVER-03`,
  `PT-ETYMON-ENTENDER-COMPREENDER-03`, `PT-LEX-IDADE-02`/`-ETYMON-03`/
  `-GRAMMAR-04`, `PT-PRAGMATICS-TENHO-FALADO-04`, `PT-LEX-FORMAL-PRACTICE-02`,
  `PT-ETYMON-DIAS-2-02`, `PT-GRAMMAR-DIAS-2-03`,
  `PT-ETYMON-MEIO-DIA-MEIA-NOITE-02`, `PT-LEX-C03-PRACTICE-02`,
  `PT-NOTICE-C03-PRACTICE-03`, `PT-LEX-C04-PRACTICE-02`,
  `PT-NOTICE-C04-PRACTICE-03`, `PT-NOTICE-C05-PRACTICE-04`, `PT-LEX-MAO-02`,
  `PT-GRAMMAR-MAO-04`, and the three `PT-*-GOSTAR-*` atoms. Each is genuinely
  practised in the lesson body, not merely listed.
- **Regional variation, stated plainly rather than papered over.** Portugal
  commonly says *jogar à bola*, *jogar às cartas*; Brazil, and northern
  Portugal, take the object straight — *jogar bola*. And Portugal hangs the
  reflexive on the back (*encontramo-nos*, with the *-s* of *encontramos*
  dropping) where Brazil more often fronts it (*nós nos encontramos*).
- **Budgets.** 9 new atoms in Chapter 21 and 8 in Chapter 22, both inside
  `core/chapter-policy.json`'s advisory `maxNewAtomsPerChapter` of 12. Every
  lesson's *computed* duration is under the 300-second gate (279–296s), all seven
  derive as `voice`/drivable, and no table exceeds three columns. Sequence
  numbers run 750–810, continuing 670–740 without a gap.
- Wired: `curriculum.json` (segments `PT-PATH-025`/`026` on
  `SPINE-SAY-WHAT-I-DO`, the seven concepts dropped from its `omits`, extensions
  `PT-EXT-025`/`026-LANGUAGE-SPECIFIC`), `chapters.json`,
  `core/book-generation.json`, `book/book.tex`, and the regenerated book
  chapters, narration and modality manifest.

