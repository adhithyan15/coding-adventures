## The final verb tranche: carrying, buying, waiting, meeting, playing, getting, answering

Italian authors the last seven core verbs no track in the corpus taught, moving
from **14 of 40** to **21 of 40**. With Spanish, Latin and Portuguese landing the
same seven in parallel, **every one of the 40 core verbs is now realized
somewhere** — `verbCoverage` reports `universallyMissing: []` for the first time.

Seven one-verb lessons in **two** chapters again, for the same reason as
Chapters 18–19: seven lessons introduce 21 atoms, more than
`maxNewAtomsPerChapter: 12` allows in one chapter.

- **Chapter 20 — Carrying, Buying, Waiting, Meeting** (`IT-C20-portare`,
  `IT-C20-comprare`, `IT-C20-aspettare`, `IT-C20-incontrare`; concepts
  `VERB-BRING`, `VERB-BUY`, `VERB-WAIT`, `VERB-MEET`). Four regular *-are*
  verbs, so the endings cost nothing and the whole chapter's weight sits on the
  roots. Etymological thread: three of the four are a plain Latin verb with a
  small word welded to the front — *com-*, *ad-*, *in-*. 12 new atoms, at the
  chapter budget.
- **Chapter 21 — Two Ways to Play, and Two Verbs That Bend** (`IT-C21-giocare`,
  `IT-C21-ottenere`, `IT-C21-rispondere`; concepts `VERB-PLAY`, `VERB-GET`,
  `VERB-ANSWER`). 9 new atoms.
- **The signature: *giocare* against *suonare*.** English has one verb for
  playing a game and playing an instrument; Italian has two, from two unrelated
  Latin words — *iocus* "a jest" (→ *joke*, *jocular*, *jeopardy* ← *jeu parti*)
  and *sonus* "a noise" (→ *sound*, *sonata*, *unison*, *dissonant*,
  *counterpoint*). The split is not a rule to memorise; it is the merger English
  performed and Italian never did.
- **A correction the brief did not contain.** The tranche brief asserted
  *comprare* ← *comparāre* → English *compare* as "a non-obvious true link". It
  is false. Latin had **two** verbs spelled *comparāre*: one on *parāre* "to
  make ready, procure" (→ Italian, Spanish and Portuguese *comprare/comprar*)
  and one on *pār* "equal" (→ English *compare*, *par*, *parity*, *peer*,
  *pair*, *umpire*). They are homographs, not relatives. Verified against
  Wiktionary's `comprare` entry, which derives it from `cum + parō`. The refusal
  is authored as the lesson's third atom (`IT-ETYMON-COMPRARE-04`, "The
  look-alike, taken apart"), in the same move the track already makes on
  *domandare* ≠ *demand*, and the real *parāre* family is taught in its place:
  *prepare*, *repair*, *separate*, *apparatus*, *apparel*, *emperor*, and
  Italian's own *parare* behind *parry*, *parasol* and *parachute*.
- **Reinforcement (HL09 §7) at two cadences.** Every lesson practises atoms from
  the immediately preceding one to three lessons, across the chapter seam, and
  each payoff reaches back several chapters. The tranche **rescues eleven
  pre-existing orphans** — atoms taught once and never touched again:
  `IT-LEX-MI-PIACE-02`, `IT-GRAMMAR-MI-PIACE-03` and `IT-NOTICE-MI-PIACE-04`
  (Ch. 19, via *mi piace giocare* and the extended reaching-thread);
  `IT-LEX-ETA-02`, `IT-ETYMON-ETA-03`, `IT-GRAMMAR-ETA-04` and
  `IT-GRAMMAR-ETA-05` (Ch. 14, via the silent *h* of *giochi* against *hanno*,
  and via Spanish *tengo* / Portuguese *tenho* being *tenēre*);
  `IT-LEX-PASSATO-PROSSIMO-ESSERE-02`, `IT-GRAMMAR-PASSATO-PROSSIMO-ESSERE-03`
  and `IT-PRAGMATICS-PASSATO-PROSSIMO-ESSERE-04` (Ch. 16, via *ho incontrato
  Anna* against *Anna è andata*); and `IT-LEX-MANO-02` (Ch. 17, via *a mano*).
  Chapter 21's payoff also retrieves `IT-LEX-PASSATO-REMOTO-02`,
  `IT-PRAGMATICS-PASSATO-REMOTO-04` and `IT-NOTICE-PASSATO-REMOTO-05`, and
  `IT-GRAMMAR-MANO-03` — Chapter 17's principle that an "irregular" form is a
  regular one from a system you can no longer see, applied to *risposto*.
  Measured on the committed corpus, Italian's never-revisited count falls
  **28 → 14** while its taught-atom count rises **135 → 156**. Only **three**
  new orphans appear, and all three belong to the track's final lesson, where no
  later lesson exists to reach them.
- **No forward references.** Every example uses vocabulary the track has already
  taught — *pane*, *acqua*, *vino*, *mano*, *anni*, *Marco*, *Anna* — and no
  lesson teases a later one.
- **Wiring.** `SPINE-SAY-WHAT-I-DO` gains two real path segments, `IT-PATH-024`
  and `IT-PATH-025`, and drops the seven concepts from its `omits`.
  `chapters.json` gains capability entries 20 and 21,
  `core/book-generation.json` two Italian targets, and `book/book.tex` two
  `\input` lines. Book chapters, the modality manifest and the narration export
  are regenerated; all seven lessons classify **voice**, so the whole tranche is
  drivable. Every lesson's computed duration lands between 281 s and 298 s,
  inside the 300 s gate.

