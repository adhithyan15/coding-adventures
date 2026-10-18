## Chapters 19 and 20 — the second core-verb tranche, split in two

- Added eight lessons realising eight more of the shared spine's core-verb
  concepts, verbatim. **Chapter 19, "Verbs of the Mind"**: `VERB-THINK`
  (`PT-C19-pensar`), `VERB-UNDERSTAND` (`PT-C19-entender-compreender`),
  `VERB-READ` (`PT-C19-ler`), `VERB-WRITE` (`PT-C19-escrever`). **Chapter 20,
  "Taking, Asking, Helping, Liking"**: `VERB-TAKE` (`PT-C20-tomar-pegar`),
  `VERB-ASK` (`PT-C20-perguntar`), `VERB-HELP` (`PT-C20-ajudar`),
  `VERB-LIKE-LOVE` (`PT-C20-gostar`). **No track in the corpus taught any of
  these eight before**: all eight were among the 23 core verbs that were
  entirely unrealised. Portuguese now covers **15 of the core 40**; the
  corpus-wide universally-missing list drops from 23 to 15.
- **Why two chapters and not one.** Authored first as a single eight-lesson
  chapter, it introduced 17 atoms against `core/chapter-policy.json`'s advisory
  `maxNewAtomsPerChapter` of 12. Splitting it into two four-lesson chapters
  lands at **8 and 9 atoms** — inside budget, with the corpus ramp violation
  count returning to its pre-existing 25. The alternative fixes were both worse:
  thinning the lessons would have cost real content, and raising the budget
  would have made the number disappear without making the ramp any gentler.
  Eight new verbs in one sitting genuinely is steeper than two sittings of four,
  so the budget was measuring something real. Sequence numbers (670–740) and the
  prerequisite chain are unchanged across the seam.
- **Chapter 20's signature is *gostar de*.** Portuguese does not like a thing,
  it likes *of* it — *gosto de café*, *gosto de você* — and the lesson gives the
  preposition its own Grammar Lens, because the *de* is a fossil of the verb's
  original sense: Latin *gustāre de* was to take a taste **from** what is in
  front of you. The `de + o = do` fusions are taught there too. Spanish, which
  escaped the same problem by inverting the sentence instead (*me gusta el
  café*), is supplied as self-contained contrast — never as knowledge the reader
  is presumed to arrive with.
- **Two more Portuguese-specific things get named rather than implied.** *Ler*
  is irregular in a way worth a lesson: *leio / lês / lê / lemos / leem*, the
  same broken row as *ver* and *crer*, and it is broken because Latin *legere*
  lost its *-g-* between vowels exactly as *manus* lost its *-n-* to give *mão*
  (Chapter 17). And *tomar* against *pegar* splits English "take" by destination
  — into your hand, or into you.
- **Two verbs get one lesson each, twice**, on the Chapter 18 principle:
  *entender* and *compreender* are one choice, and so are *tomar* and *pegar*.
  Splitting either pair would teach half a decision.
- Etymology, all traceable: *pēnsāre* ← *pendere* "to hang" →
  *pensive/ponder/pension/compensate/expense/pendulum/pendant/depend/suspend*,
  plus *achar* ← *afflāre* "to breathe upon"; *intendere* →
  *intend/intense/extend/pretend/tension/tent/tendon* and *comprehendere* →
  *comprehend/apprehend/prehensile/prison/prize/surprise*; *legere* "to gather"
  → *collect/select/elect/intellect/diligent/neglect/legible/lecture/legend*
  and, through French *leçon*, *lesson*; *scrībere* →
  *scribe/script/describe/prescribe/transcribe/manuscript/shrift*; *picāre* "to
  coat with pitch" → *pitch*, still sticky in *pegajoso*; *percontārī* = *per-*
  + *contus*, "to sound the water with a pole", with *pedir* ← *petere* →
  *petition/appetite/compete/impetus*; *adiūtāre* →
  *aid/aide/adjutant/adjuvant*; *gustāre* →
  *gusto/gustatory/degustation/disgust*.
- **One root is flagged as unknown and one as thin, on purpose.** *Tomar* has no
  settled Latin source — *autumāre* and an unrecorded *tumāre* are both proposed
  — and the lesson says so rather than inventing a hook. *Percontārī*'s *contus*
  left English almost nothing, and the lesson says that too: a root with no
  cousins is still worth knowing. False friend flagged: English **understand**
  belongs to neither *intendere* nor *comprehendere*; it is Germanic.
- Two spelling/sound rules taught as decoding tools rather than trivia: the
  propped-up **e-** that Iberian Romance put in front of every Latin *sc-/sp-/
  st-* word (*scrībere* → *escrever*, *schola* → *escola*, *stāre* → *estar*,
  *spērāre* → *esperar*), and Latin *i*-before-a-vowel hardening into Portuguese
  *j* (*adiūtāre* → *aiudar* → *ajudar*, and likewise *hoje*, *já*, *janeiro*).
- All eight lessons are schema v2, `voice`-modality (no sight cues, no table
  wider than two), and prerequisite- and knowledge-closed. Longest effective
  duration 285 s (`PT-C20-gostar`), shortest 264 s (`PT-C19-ler`) — every one
  under the five-minute contract.
- Wiring: `curriculum.json` gains `PT-PATH-023`/`PT-EXT-023-LANGUAGE-SPECIFIC`
  and `PT-PATH-024`/`PT-EXT-024-LANGUAGE-SPECIFIC`, both on
  `SPINE-SAY-WHAT-I-DO`, whose `omits` drops from 35 concepts to 27.
  `chapters.json` gains both ledgers. Chapter 19's payoff is `PT-C19-escrever`,
  assessing 6 of that chapter's 8 atoms (**0.75**); Chapter 20's is
  `PT-C20-gostar`, assessing 6 of that chapter's 9 (**0.67**) plus the two
  Chapter 18 *saber* atoms it genuinely re-derives. Both clear the 0.5
  representativeness floor, and the corpus count of payoffs below it stays at
  its pre-existing 24.
- To make Chapter 19 close honestly on its own, `PT-C19-escrever` now runs the
  chapter's four verbs in the *eu* form (*penso, entendo, leio, escrevo*) in
  both its guided practice and its wrap-up, and declares the two extra atoms it
  thereby assesses. Chapter 20's `PT-C20-gostar` still runs all eight, now
  named as "these two chapters" rather than one.
- Book: `ch19-mind-verbs.tex` and `ch20-taking-asking-helping-liking.tex`
  generated and `\input` into `book.tex`; the forced XeLaTeX build reports
  **zero** missing characters and zero errors, and the two remaining overfull
  boxes both pre-date these chapters (Chapters 7 and 10).

