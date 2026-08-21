# Changelog

## Writing begins with the first greeting (#12297)

- Migrate `PT-C01-ola` to the measured lesson schema and add one visible-model
  observe-and-trace microstep for **olá** and its stress accent.
- Add a separate two-minute guided copy of the same greeting. Neither step asks
  for spelling from memory, a second word, or free composition.
- Record the first two HL19 stages as a required pre-A1 extension. This closes
  only Portuguese's no-writing-at-all cliff; later writing stages remain debt.

## Portuguese chapter 1 regains its reading order (#12258)

- Add sequence 10 through 90 to the nine legacy greeting lessons, exactly in
  the order already authored in the book and immediately before Chapter 2.
- Remove the missing-sequence and false forward-reference debt caused by
  alphabetical fallback. Also remove `PT-C03-eu`'s stray claim to review the
  Chapter 4 farewell `adeus`; the lesson never reviews it and precedes it.

## Chapters 23-26 — the pre-A1 noun tranche

Fifteen everyday-noun lessons across four new chapters, filed under pre-A1
spine nodes, authored to extend the HL09 §3.1 level-gate vocabulary count and
to measure what it actually moves:

- **Chapter 23, "Asking for Something to Drink"** (`SPINE-POLITE-REQUEST-REPAIR`,
  3 lessons): **café** (← Arabic *qahwa*, via Turkish *kahve* — a loanword,
  not Latin, arriving in the same wave as Italian *caffè* and French *café*),
  **chá** (← Cantonese *chá* via Macau — Portuguese is the one Western
  European language that did NOT take the Hokkien/maritime *te* route that
  gave English *tea*, French *thé*), **leite** (← Latin *lac, lactis*, plain
  inheritance — the one drink of the three that is not a loanword).
- **Chapter 24, "Cheese, Egg, Rice, and Sugar"** (`SPINE-POLITE-REQUEST-REPAIR`,
  4 lessons): **queijo** (← *cāseus*, true cousin of English *cheese* —
  French/Italian replaced it with *fōrmāticum*, "moulded"), **ovo** (←
  *ōvum*, barely worn down in two thousand years), **arroz** and **açúcar**
  (both ← Arabic, the *al-Andalus* thread Chapter 13's *azul* opened —
  *arroz* keeps its fused Arabic article, *açúcar*'s cedilla reaches back to
  Chapter 17's *cabeça*).
- **Chapter 25, "The People You Introduce"** (`SPINE-EXCHANGE-NAMES`,
  4 lessons): **amigo/amiga** (← *amīcus*, built on *amāre* "to love" —
  closing the loop Chapter 3's *prazer*, ← *placēre* "to please," opened),
  **família** (← *familia*, which in Classical Latin named the whole
  household under one head, servants included — not specifically blood
  relatives), **homem** (← *homō*, "human being," narrowed to "man" in
  Portuguese/Spanish/French alike), **mulher** — the chapter's payoff:
  Portuguese/Spanish keep *mulier*, French built *femme* on a different
  Latin word (*fēmina*), and Italian *donna* comes from a third again
  (*domina*, "lady of the house").
- **Chapter 26, "The Body, Asking How You Are"** (`SPINE-CHECK-WELLBEING`,
  4 lessons): **olho** and **orelha** (← *oculus*/*auricula*, both showing
  the same Vulgar Latin **-c'l- → -lh-** fold Chapter 13's *vermelho* (←
  *vermiculus*) already demonstrated — and *orelha*/Spanish *oreja* rerun
  the identical PT-*lh*/ES-*j* split Chapter 5's *trabalhar*/*trabajar*
  showed on a verb), **boca** (← *bucca*, "cheek" in Classical Latin,
  generalised to "mouth" in Vulgar Latin — the real Latin word for mouth,
  *ōs, ōris*, survives only in *oral*), **coração** — the chapter's payoff:
  built on *cor, cordis* plus an augmentative suffix, so its nasal *-ão*
  was **grown**, unlike *pão*'s and *irmão*'s, which reached the identical
  ending by **losing** a sound.

**Measured before and after** with `buildCurriculumGapReport` / the HL09 §3.1
level gate:

  vocabulary at or below pre-A1 (of 300)   32 -> 47   (shortfall 268 -> 253)
  track vocabulary (any level)             72 -> 87
  reinforcement shortfall (pre-A1)         27 -> 5
  attained / inProgressAt              null / pre-A1  (unchanged)

Fifteen new word lessons moved the vocabulary count by exactly fifteen —
`vocabularyOf()` counts distinct `headword:` strings on `word`/`phrase`
lessons, one per lesson, confirming what the Hindi/Arabic/Tamil tranches
already measured: there is no bulk credit, and closing pre-A1 on vocabulary
alone still needs roughly 253 more lessons of this shape in this track.

Reinforcement is where the tranche pays for itself. All 22 pre-A1 atoms this
tranche could honestly reach — those realized under
`SPINE-POLITE-REQUEST-REPAIR`, `SPINE-EXCHANGE-NAMES` and
`SPINE-CHECK-WELLBEING`, the three nodes these chapters attach to — are now
revisited at least twice: `PT-GRAMMAR-TRABALHAR-04`, `PT-LEX-FALO-PORTUGUES-02`/
`-03`, `PT-NOTICE-C05-PRACTICE-04`, `PT-SOUND-TRABALHAR-02` (Chapter 23's
payoff reaches back to Chapter 5); `PT-GRAMMAR-IDADE-04`, `PT-GRAMMAR-PAIS-03`,
`PT-GRAMMAR-PRAZER-04`, `PT-GRAMMAR-TENHO-FALADO-03`, `PT-LEX-C03-PRACTICE-02`,
`PT-LEX-IDADE-02`, `PT-NOTICE-C03-PRACTICE-03`, `PT-PRAGMATICS-TENHO-FALADO-04`,
`PT-SOUND-PRAZER-02` (Chapter 25); `PT-GRAMMAR-MAIS-OU-MENOS-04`,
`PT-GRAMMAR-MAO-04`, `PT-LEX-C02-PRACTICE-02`, `PT-LEX-FORMAL-PRACTICE-02`,
`PT-LEX-MAO-02`, `PT-NOTICE-C02-PRACTICE-03`, `PT-SOUND-CABECA-02`,
`PT-SOUND-MAIS-OU-MENOS-02` (Chapter 26). The remaining 5 thin atoms
(`PT-SOUND-DE-NADA-02`, `PT-LEX-C04-PRACTICE-02`, `PT-NOTICE-C04-PRACTICE-03`,
`PT-SOUND-ATE-AMANHA-02`, `PT-SOUND-ATE-BREVE-02`) sit under
`SPINE-COURTESY-THANK` and `SPINE-TAKE-LEAVE`, which this tranche does not
touch. Closing the loop cost real cross-chapter chaining, not just a payoff
dump: several middle-of-chapter atoms (e.g. `PT-LEX-CHA-02`, `PT-LEX-ARROZ-02`,
`PT-LEX-HOMEM-02`) only reach two revisits because the **next** chapter's
opening lessons reach back into the previous one, not just the payoff lesson.

**A finding independently confirmed, not new**: the seven pre-A1 spine nodes
are all social speech acts and own no concept for a concrete object sitting
in front of the learner. This tranche stayed inside the categories that DO
have an honest home — food/drink requested (`SPINE-POLITE-REQUEST-REPAIR`),
people introduced (`SPINE-EXCHANGE-NAMES`), body words extending a wellbeing
check-in (`SPINE-CHECK-WELLBEING`) — rather than attempting the
household-object framing (house/door/room/chair) that the Hindi and Tamil
tranches used and the Arabic tranche declined.

**Corrected against sources during authoring**: *coração* is not a bare
continuation of Latin *cor* — it is *cor* plus an augmentative suffix
(descended from *-atiōne*), the same building pattern as Spanish *corazón*,
which French *coeur* and Italian *cuore* did not follow. *Boca*'s Latin
ancestor *bucca* meant specifically "cheek" in Classical Latin; the actual
Classical word for "mouth" was *ōs, ōris*, which survives in English only via
*oral*/*orifice*. *Mulher*'s Romance sisters do not share one root: French
*femme* is from *fēmina*, a different Latin word from *mulier*, and Italian
*donna* is from a third again, *domina* ("lady of the house") — not from
*fēmina* either, a detail easy to get wrong by assuming *donna* and *femme*
share a root because they mean the same thing.

**Font check**: compiled with XeLaTeX before and after authoring; zero
`Missing character` warnings, 168 pages. No new script or glyph is used —
every new word is Latin-script, matching the track's existing font.

**Variety**: `variety: standard-contemporary`, matching every existing
Portuguese lesson. Word choices (café, chá, leite, queijo, ovo, arroz,
açúcar, amigo/amiga, família, homem, mulher, olho, orelha, boca, coração)
are identical in European and Brazilian Portuguese; no European/Brazilian
split word (e.g. *sumo*/*suco*) was needed.

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

## Chapter 18 — Verbs at the Core

- Added seven lessons realising the shared spine's core-verb concepts, verbatim:
  `VERB-BE` (`PT-C18-ser-estar`), `VERB-HAVE` (`PT-C18-ter-haver`), `VERB-GO`
  (`PT-C18-ir`), `VERB-COME` (`PT-C18-vir`), `VERB-SAY` (`PT-C18-dizer`),
  `VERB-SEE` (`PT-C18-ver`), and `VERB-KNOW` (`PT-C18-saber-conhecer`). Before
  this the track realised **zero** canonical `VERB-*` concepts and
  `SPINE-SAY-WHAT-I-DO` was omitted whole; it now carries `PT-PATH-022` and
  seven fewer omissions.
- **Two verbs get one lesson each, twice, and that is the point.** *Ser* and
  *estar* are one concept — how Portuguese says "to be" — so they share the
  `VERB-BE` lesson; *saber* and *conhecer* likewise share `VERB-KNOW`. Splitting
  either pair would teach half a choice.
- What is new against Chapters 14–16, which already taught *ter*'s and *ser*'s
  forms: the **divergences**, named rather than implied. Portuguese files
  marital status under *ser* (*sou casado*) where Spanish uses *estar*; *ficar*
  quietly does locations. Portuguese built its compound past on *ter* (*tenho
  falado*) while Spanish kept *haber* for the job — the same two inherited verbs,
  opposite winners. And *haver*'s surviving shapes, `há` "there is" and `há três
  anos` "three years ago", plus `ter que` for obligation.
- Etymology, all traceable: *esse* → *is/am/essence/present/absent/interest*;
  *stāre* → *state/status/estate/circumstance*; *habēre* →
  *habit/inhabit/exhibit/prohibit/able/debt*; *īre* →
  *exit/transit/initial/ambition/perish/preterite* with *vādere* →
  *evade/invade/wade*; *venīre* → *venue/advent/convene/prevent/event/souvenir*;
  *dīcere* → *dictate/verdict/edict/index* and *iūdex* → *judge*; *vidēre* →
  *vision/evident/provide/prudent/survey/envy*; *sapere* →
  *savour/sapient/insipid*; *cognōscere* → *cognition/recognise/connoisseur*.
- Three false friends flagged: *estar* is unrelated to English **stare**;
  English **have** is not from *habēre* (it is Germanic, from the "seize" root
  behind *capere* → **capture**); English **very** is not from *vidēre* but from
  *vērus*, the *vērum* inside **verdict**.
- Two Portuguese-internal traps taught explicitly: *vem* against *vêm*, where an
  accent carries number exactly as in *tem*/*têm*; and **vimos**, which is "we
  saw" for *ver* and "we come" for *vir*, disambiguated only by context.
- All seven lessons are schema v2, `voice`-modality (no sight cues, no table
  wider than two), and prerequisite- and knowledge-closed. Longest effective
  duration 273 s (`PT-C18-saber-conhecer`), shortest 216 s (`PT-C18-vir`) —
  every one under the five-minute contract.
- `chapters.json` gains the Chapter 18 ledger. The payoff is
  `PT-C18-saber-conhecer`, whose `assesses` is exactly its own
  `practises.knowledge`: all fourteen of the chapter's atoms, scoring 1.00
  against the 0.5 representativeness floor. The chapter introduces 14 atoms
  against `core/chapter-policy.json`'s advisory `maxNewAtomsPerChapter` of 12 —
  recorded, not fixed by thinning the lessons; the Latin core-verb chapter sits
  at 16 for the same reason.
- Book: `ch18-core-verbs.tex` generated and `\input` into `book.tex`; the forced
  XeLaTeX build reports **zero** missing characters, and the two overfull boxes
  in the log both pre-date this chapter (Chapters 7 and 10).

## HL05 chapter capabilities — Chapters 2–17

- Added `chapters.json`, the track's HL05 capability ledger, with 16 entries
  covering every Portuguese chapter that owns a `core/book-generation.json`
  target.
- Each entry declares a first-person `canDo`, the shared spine nodes the chapter
  realises (derived from `curriculum.json` path segments), and a `payoff` naming
  the lesson that proves the claim, its kind, a one-line summary, and the
  knowledge atoms it exercises. Every `assesses` list is exactly the payoff
  lesson's own `practises.knowledge` — nothing invented.
- Payoff selection: Chapters 2–5 use their terminal `practice-mix`. Chapters
  6–17 have no practice lesson, so the payoff is the chapter's last lesson by
  sequence, which is where its recombination and wrap-up recall live.
- **Skipped, deliberately:** Chapter 1. Its lessons are still schema v1 with no
  declared `practises.knowledge`, so no payoff can be claimed honestly, and it
  owns no book-generation target either. The absence is tracked debt; a stub
  would have destroyed the HL05 gap report's signal.
- **Representativeness risk** against the 0.5 threshold in
  `core/chapter-policy.json`: Chapter 2 scores 0.25 (4/16). It is the only
  chapter in the corpus with two `practice-mix` lessons — the full casual
  exchange at sequence 160 and the register drill `PT-C02-formal-practice` at
  170. The terminal-consolidation rule selects the register drill, which
  practises only four atoms; the earlier `PT-C02-practice` would score 0.94.
  Recorded rather than fixed by inflating `assesses` beyond what the terminal
  lesson actually practises. Every other chapter scores 0.50 or above
  (Chapter 16 at 0.50, Chapter 17 at 0.67, the rest 1.00).

## Warning-free 105-page edition — 2026-08-03

- Added `\raggedbottom` so deliberately short micro-lesson pages keep natural
  bottoms instead of producing eleven underfull vertical boxes.
- Refined copy flow in six canonical lessons, using clearer sentence boundaries,
  a shorter resumable heading, and explicit warm-up paragraph breaks to remove
  six overfull and two underfull horizontal boxes without dropping content.
- Regenerated the affected chapters and app-verified source fingerprints. All
  lessons remain prerequisite-closed and below five minutes.
- The forced XeLaTeX build now reports zero missing glyphs, overfull or
  underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings
  across all 105 pages.

## Canonical book Chapters 2–17 — 2026-08-03

- Migrated all 50 lessons in Chapters 2–17 to strict schema v2 with unique
  sequence, shared-spine anchors, prerequisite-closed knowledge atoms, typed
  blocks, explicit coverage metadata, and computed durations below five
  minutes.
- Added sixteen deterministic LaTeX generation targets and matching Language
  Ladder hash tests. The downloadable book and app now consume the same lesson
  AST and independently agree on every chapter's canonical source fingerprint.
- Expanded the book from Chapter 1 to all seventeen chapters (105 pages) while
  preserving the original vocabulary, grammar, usage, and etymological depth.
- Preserved Arabic `حتى` beside *ḥattā* and render it with the same vendored
  Noto Naskh Arabic font used by the Arabic-script tracks, eliminating a
  learner-visible missing-glyph defect without weakening the canonical text.
- Recorded the expanded layout baseline in HL-B17: six overfull and thirteen
  underfull boxes remain, with no missing glyphs, duplicate destinations,
  Hyperref warnings, or LaTeX warnings.

## Sub-five-minute remediation — 2026-08-02

- Corrected eighteen declared five-minute estimates whose lesson bodies already
  compute below 300 seconds.
- Replaced five computed violations with five prerequisite-ordered support
  lessons spanning question register, *ser* suppletion, *ser/estar* meaning,
  and the inherited/re-borrowed *caput* pair.
- Preserved the full vocabulary, grammar, etymology, and cross-language depth.
  The shared report now measures zero Portuguese duration violations.
- `PT-C17-mao` at 293 computed seconds is the tightest remaining Portuguese
  lesson and should be watched during copy edits.

## Chapter 17 — The body: the head that stayed a head, and the *n* that dissolved

- **Chapter 17 authored** (`PT-C17-cabeca`, `-cabeca-caput`, `-mao`) — the **body**, the theme the
  parallel-track roadmaps name next.
- **a cabeça / the *caput* map** (`PT-C17-cabeca`, `-cabeca-caput`): the
  chapter's role in the four-way set is that
  **Portuguese kept the Latin word**. *Cabeça* ← Late Latin *capitia* ← ***caput***,
  where French and Italian replaced "head" with a **pot** (*testa*) and German
  with a **cup** (*Kopf*) — independently of each other. The five-language table
  is the payoff: *cabeça*/*cabeza* still call the head a head; the other three
  swapped in a vessel. Iberia didn't sit the shift out entirely, though — it
  **narrowed** the pot-word: *testa* is alive in Portuguese and means the
  **forehead**. Stated that way because "sat out the joke" was an absolute a
  reader could falsify with any dictionary.
  - Also teaches the **inherited-vs-reborrowed doubling**, which is worth
    recognising generally: *cabeça* is *caput* worn down by everyday use, while
    *capital*, *capítulo* and *capitão* are the **same root re-imported intact**
    from written Latin. One root, entering the language twice.
  - The **ç** is explained as a reading note rather than a handwriting one, since
    **the Portuguese track has no writing chapters** — checked rather than
    assumed, after an early draft cited a `PT-W01` lesson that does not exist.
- **a mão** (`PT-C17-mao`): ← *manus*, with the intervocalic ***-n-* dissolved**
  and the preceding vowel left nasal — which is exactly what the **til** records.
  Presented as **systematic, not a one-off**: *lua* ← *lūna*, *boa* ← *bona*,
  *pão* (Ch. 11). Spanish kept them all (*mano, luna, pan*), and this single
  change is a large part of why written Portuguese and Spanish look so similar
  and sound so different.
  - Feminine for the same reason as Italian's *la mano* — *manus* was
    **fourth-declension feminine** — with the note that Portuguese is less
    startling about it only because *mão* lacks a giveaway *-o*.
  - The *pão* row cites the Latin form **as Ch. 11 already gives it**, rather
    than introducing a competing one.

## Chapter 16 — *ser* and *estar*: sitting against standing

- **Chapter 16 authored** (`PT-C16-ser`, `-ser-roots`, `-ser-vs-estar`,
  `-ser-estar-meaning`). Portuguese had **no
  "to be" lesson at all** — no *ser*, and *estar* only overheard inside Ch. 2's
  *Como está?*. This is the largest single gap in the track, now closed.
- **ser / its roots** (`PT-C16-ser`, `-ser-roots`): present, preterite and
  imperfect, and then why *sou*,
  *fui* and *era* have nothing in common — **two Latin verbs but three stems**:
  *esse* (for *sou/é/são* and for *era*), *esse*'s **own** ancient perfect
  ***fuī*** (PIE \**bʰuH-*, the root of English **be** and German **bin/bist**),
  and — the surprise — the infinitive ***ser*** from ***sedēre***, "**to sit**"
  (also English *sedentary*, *session*, and Portuguese *sede* "seat,
  headquarters"). The count is made explicit, because *esse* was **already
  suppletive in Latin**: *sum* and *fuī* were different roots long before
  Portuguese existed.
  - **The headline fact: *ser* and *ir* share their entire preterite.**
    *fui/foste/foi/fomos/foram* is both "I **was**" and "I **went**" — *fui
    professor* against *fui ao Brasil*. The lesson is careful about **who
    borrowed from whom**: *fuī* was *ser*'s all along, and it was ***ir*** that
    came in empty-handed, Latin *īre*'s own perfect (*iī*, *īvī*) having eroded
    to nothing. (This matches `ES-C14-ser-ir-preterite`, which tells it the same
    way.) Framed as a **gift to the learner**: *ir*'s preterite needs no separate
    lesson — which is also why this chapter closes the set **without** the
    motion-verb lesson French and German required.
- **ser vs estar / meaning shifts** (`PT-C16-ser-vs-estar`,
  `-ser-estar-meaning`): *estar*'s forms named at last, with
  the spoken reductions (*tou bem*, *tá bem*) flagged as recognise-don't-write,
  and *tá bom* labelled as characteristically **Brazilian** (*tá bem* is heard in
  both) rather than left to imply that an otherwise European-leaning lesson says
  it.
  - The rule (identity/origin/profession/time → *ser*; location/health/mood/
    weather → *estar*) is given, then immediately stress-tested with ***ele está
    morto*** — *estar*, for the least temporary state there is. So the test
    offered is not permanence but **naming what a thing is** vs **the condition
    it ended up in**.
  - ***ser* ← *sedēre*, to SIT. *estar* ← *stāre*, to STAND.** Sitting is
    settled, where a thing belongs; standing is a posture you happen to be in —
    a **question to ask** rather than two lists to memorise. But the lesson says
    plainly that this is a **mnemonic, not the cause**: the "essence" sense comes
    from ***esse***, *sedēre* mostly supplied *forms* (infinitive, future,
    conditional), and the real driver was *stāre*'s locative/resultative sense
    pulling away from plain *esse*. The picture is useful **because** it sits
    downstream of that.
  - Minimal pairs presented as **a distinction to use, not an error to avoid**:
    *ela **é** bonita* ("she is beautiful") vs *ela **está** bonita* ("she looks
    beautiful — tonight"); *a sopa **é** boa* vs *a sopa **está** boa*.
  - Ends with the comparative table this parallel-4 chapter was built to earn:
    **three** Romance outcomes from the same two Latin verbs — Iberia (PT, ES)
    **split** them, France **absorbed** one into the other, Italy kept **both,
    overlapping** — with **German** marked out as a *separate* story, its three
    roots coming down **directly** from PIE rather than by way of Latin. Stated
    carefully rather than as "roots that were never Latin", because \**h₁es-* is
    exactly the root Latin's own *esse* came from: German and Latin are
    **cousins** here, not parent and child.
- **Both lessons strike through *vós*** (*~~vós sois~~*, *~~vós estais~~*) and
  drill **vocês são / vocês estão** instead. *Vós* is dead in both EP and BP
  outside northern dialect and liturgical register; printing it unmarked in a
  paradigm — and rehearsing it in Guided Practice — would have taught a form the
  learner will never need to produce.
- Parallel to `ES-C09-ser-vs-estar`; the Spanish lesson is the cousin to teach
  alongside it.

## Chapter 15 — The past Portuguese kept, and the compound that means something else

- **Chapter 15 authored** (`PT-C15-preterito-perfeito`, `-tenho-falado`): the
  everyday past, plus the construction English speakers reliably mistranslate —
  reviewing Ch.5/14 via `reviews_of`.
- **pretérito perfeito** (`PT-C15-preterito-perfeito`):
  *falei/falaste/falou/falámos/falaram* ← Vulgar Latin perfect **\*fabulāvī** (the same source
  as Spanish *hablé*), **one word, no auxiliary** — the *-āv-* dissolved and left a
  stressed final vowel behind. The lesson's interest is comparative rather than
  morphological: **all five languages inherited this tense, and only Portuguese and
  Spanish still use it in daily speech.** French exiled it to literature (*il
  parla*), German pushed it aside (*sagte*), northern Italy dropped it (*parlò*) —
  and the two at the **western edge** simply kept it.
- **tenho falado** (`PT-C15-tenho-falado`): *ter* + participle, which does **not**
  mean "I have spoken." It means "I **have been** speaking" — **repeatedly, over a
  recent stretch, possibly still going**; a single finished act is *falei*. The
  explanation is the chapter's payoff: Portuguese assembled the **same** *have* +
  participle machine as French and Italian, but ***falei* had never vacated the
  plain-past slot** — with **no vacancy**, the new construction drifted into an
  **iterative** job instead. Same parts, same era, same family; different outcome,
  because of what was already in the way. Flagged explicitly as a very common  English-speaker error in Portuguese.
- Taxonomy: namespaced `PT-PAST-SIMPLE`, `PT-PAST-COMPOUND-ITERATIVE`.

## Chapter 14 — ter, and holding your years

- **Chapter 14 authored** (`PT-C14-ter`, `-idade`): the workhorse verb, and the
  place Portuguese and Spanish broke ranks with the rest of Romance — reviewing
  Ch.5/9/10/11/12/13 via `reviews_of`.
- **ter** (`PT-C14-ter`): *tenho/tens/tem/temos/têm*, with the **-nh-** already met
  in *vinho* and *amanhã*, and a **circumflex** that alone separates *tem* ("he
  has") from *têm* ("**they** have") — an accent doing **grammar**, not sound. The
  chapter's subject is the **Iberian swap**: Portuguese and Spanish **alone** took
  Latin ***tenēre*** "to **hold**" as their everyday word for *have*, while French
  and Italian kept *habēre* — and Portuguese's own *habēre* survivor, **haver**,
  was **demoted** to an auxiliary and to *há* ("there is"). The two verbs swapped
  jobs on the peninsula. English kept the *tenēre* family as borrowings: *tenant*
  (one who **holds** land), *retain*, *maintain*, *tenacious*, *tenure*, *contain*.
- **tenho vinte anos** (`PT-C14-idade`): age via *ter*, never *ser* — and because
  *ter* came from "to hold," the sentence literally says "**I hold twenty
  years**," the most physical version of the idiom in the five languages. *Ano* ←
  *annus* then shows the **two Iberian outcomes of Latin -nn-**: Portuguese
  **simplified** it to a plain *n*, Spanish **palatalized** it into **ñ** (*año*) —
  which is where the *ñ* came from in the first place. Closes on the layered table:
  Germanic **is** its years, French and Italian **have** them, Iberian Romance
  **holds** them.
- Taxonomy: namespaced `PT-VERB-HAVE`, `PT-AGE`.

## Chapter 13 — Colours

- **Chapter 13 authored** (`PT-C13-preto-branco`, `-vermelho-azul`): the strangest
  colour set of the four tracks, reviewing Ch.11/12 via `reviews_of`.
- **preto & branco** (`PT-C13-preto-branco`): Portuguese keeps **two blacks** —
  literary **negro** ← *niger* (the expected inheritance, = French *noir*, Italian
  *nero*), and the everyday **preto** ← Latin ***pressus*** ("pressed, compact"), a
  colour named for how **dense** it is: dense → thick → dark (cf. *press*,
  *pressure*, *compress*). **Branco** ← Germanic ***blank*** ("shining"), arriving
  with the Suevi and Visigoths, and showing the Portuguese **l→r** signature
  (*blanc-* → *branc-*, as *plaza* → *praça*); *albus* survives in **alvorada/alva**
  ("dawn").
- **vermelho & azul** (`PT-C13-vermelho-azul`): the chapter's payoff. Every other
  Romance language builds "red" on PIE *h₁rewdʰ-*; Portuguese instead says
  **vermelho** ← ***vermiculus***, "**little worm**" — the **kermes** scale insect
  harvested off oak trees and crushed for scarlet dye, the **dye's name becoming the
  colour's name** (→ English **vermilion**; *vermis* → *vermin*). The old root
  survives only in *rubro* and *ruivo*. **Azul** ← Arabic ***lāzaward*** ("lapis
  lazuli") ← Persian *lāžward*, the *l-* dropped as if it were the article — the
  al-Andalus thread (first opened by Spanish *hasta*) reaching a **basic colour**.
  Closing observation: **not one** of Portuguese's four basic colours is a plain
  inherited Latin colour word.
- Taxonomy: namespaced `PT-COLOUR-BLACK-WHITE`, `PT-COLOUR-RED-BLUE`.

## Chapter 12 — Numbers 11–20

- **Chapter 12 authored** (`PT-C12-numeros-11-15`, `-16-20`): the teens, atom-first,
  reviewing Ch.6/Ch.11 via `reviews_of`.
- **onze–quinze** — Portuguese kept only **five** fused teens, **fewer than any
  sister**. The shared **-ze** is *decem* ("ten") worn thin, the same ten inside
  **dez** and **dezembro** (Ch.9); *catorze* keeps the *c-* where French says
  *quatorze* and Spanish *catorce*.
- **dezesseis–vinte** — the distinctive move: Portuguese **breaks earliest of all,
  at 16**, and rebuilds from living words — *dez* + **e** + *seis* = literally
  "**ten AND six**," with the **and still audible** (*dezoito* swallows it before
  the vowel of *oito*). Nothing to memorise, only to assemble.
- Includes the three-sister comparison — **PT breaks at 16**, French and Italian at
  17 — and the note that German never breaks at all. *vinte* ← *vīgintī*.
- Taxonomy: namespaced `PT-NUM-11-15`, `PT-NUM-16-20`.

## Chapter 11 — Food (bread, water, wine)

- **Chapter 11 authored** (`PT-C11-pao`, `-agua-vinho`): the everyday table trio,
  atom-first, reviewing Ch.10/Ch.1 via `reviews_of`.
- **pão** ("bread") ← *pānis*, worn to a single **nasal** syllable (*pānis → pão*)
  — the **same nasalizing erosion** that made *pai* from *pater* and *mãe* from
  *māter*; tricky plural **pães**. Root still gives *companion/pantry*.
- **água / vinho** — **água** ("water") **kept** *aqua*'s body (unlike French
  *eau*); **vinho** ("wine") ← *vīnum* carries Portuguese's signature **-nh-** (the
  palatal *ny*, = Spanish **ñ**, met before in *amanhã*) → *wine/vine/vinegar*.
- Taxonomy: namespaced `PT-FOOD-BREAD`, `PT-FOOD-DRINKS`.

## Chapter 10 — Family

- **Chapter 10 authored** (`PT-C10-pais`, `-irmaos`): the immediate family,
  atom-first, reviewing Ch.9/Ch.1 via `reviews_of` — with **two Iberian twists**.
- **pai / mãe** — the **most worn-down** parents of any sister: Latin *pater / māter*
  lost the intervocalic *-t-* entirely (*pater → pae → pai*; *mãe* went nasal). The
  masculine plural **os pais** means "the parents" (literally "the fathers"), with a
  spelling-trap note vs *o país* ("the country").
- **irmão / irmã** — the surprise: **not** from *frāter* but from Latin **germānus**
  "of the same stock" (*frāter germānus* "full brother," the *frāter* later dropped).
  Ties to the English cousins **germane** and **German** (the people-name), and to
  the identical Spanish swap *hermano / hermana*.
- Taxonomy: namespaced `PT-FAMILY-PARENTS`, `PT-FAMILY-SIBLINGS`.

## Chapter 9 — Months & seasons

- **Chapter 9 authored** (`PT-C09-meses`, `-estacoes`): the calendar year, atom-first,
  reviewing Ch.6–8 via `reviews_of`, with Spanish twins supplied.
- **The months** (the *-eiro* ending ← *-arius*, the same suffix as *feira*): the
  god/emperor parade, with the payoffs — *março* is **Mars**, and *setembro–dezembro*
  are the Latin **7–10** you can hear in *sete/dez* (Roman year began in March;
  *julho/agosto* ← Julius/Augustus were inserted).
- **The seasons**, with a twist: *primavera* = "**first spring**" (*prima* + *vēr*),
  and **verão** ("summer") also grew from *vēr* "spring" — via *vēranum*
  "spring-like/warm," so Portuguese's summer-word literally came from *spring*.
  *outono* ← *autumnus* shows the *au→ou* softening (cf. *oito/noite*); *inverno* ←
  *hibernum*.
- Taxonomy: namespaced `PT-MONTHS`, `PT-SEASONS`.

## Chapter 8 — Time & the clock

- **Chapter 8 authored** (`PT-C08-hora`, `-meio-dia-meia-noite`): telling the
  time, atom-first, reviewing Ch.6–7 via `reviews_of`.
- **hora** ← Latin *hōra* ← Greek *hṓrā* → *hour* (silent *h-* kept, as in English).
  Unlike Italian, Portuguese keeps **horas** explicit: *é uma hora* (singular) /
  *são duas horas* (plural), reusing the *é/são* split and the *dois/**duas***
  gender from the numbers.
- **meio-dia / meia-noite** — noon/midnight = *meio/meia* ("half/middle," ←
  *medius/media*) + *dia* / *noite*. The payoff: *noite* ← *noctem* carries the
  **ct→it shift** taught in the numbers chapter (*noctem → noite*, exactly like
  *octō → oito*) — a direct callback. *meia*-noite feminine, *meio*-dia masculine.
- Taxonomy: namespaced `PT-TIME-HOUR`, `PT-TIME-NOON-MIDNIGHT`.

## Chapter 7 — Days of the week

- **Chapter 7 authored** (`PT-C07-dias-1`, `-dias-2`): the seven days, atom-first,
  reviewing Ch.6 via `reviews_of` — and the **single most distinctive fact in
  Portuguese**.
- **dias-1** (segunda–sexta): Portuguese is the **only Romance language to drop the
  pagan planet-gods** and **number** its weekdays as *feiras* (← *fēria*
  "feast-day," cousin of English *fair*). Bishop **Martinho de Braga** (6th c.)
  renamed them; the ordinals are built from the very Ch.6 numbers — *quarta-feira*
  "4th" (← *quatro*), *quinta* (← *cinco*), *sexta* (← *seis*) — a direct callback.
  Everyday speech drops the *-feira* ("*Até sexta!*").
- **dias-2** (sábado, domingo): the two days that **kept** religious names —
  *sábado* ← *Sabbatum* (the Sabbath, shared with all the Romance sisters),
  *domingo* ← *(diēs) Dominica* "the **Lord's** day." And the loop closes: because
  the Church counted **domingo as the first day**, Monday becomes the *second* —
  *segunda-feira*.
- Taxonomy: namespaced `PT-DAYS-WEEKDAYS`, `PT-DAYS-WEEKEND`.

## Chapter 6 — Numbers 1–10

- **Chapter 6 authored** (`PT-C06-numeros-1-5`, `-numeros-6-10`): counting to ten,
  atom-first, each ~4–5 min, reviewing Ch.5 via `reviews_of`; Spanish twin supplied
  for each.
- **Two Portuguese signatures.** (1) **Gender survives on "two"**: *dois* (masc.) /
  *duas* (fem.), a direct survival of Latin *duo/duae* — Spanish levelled this to a
  single *dos* centuries ago. (2) The **ct→it shift**: Latin *octō → oito*, the same
  change that gives *noite* (← *noctem*) and *leite* (← *lactem*) — contrasted with
  Spanish's *ct→ch* (*ocho*). Plus the nasal *um*.
- **6–10** (*seis/sete/oito/nove/dez*) carry the **setembro–dezembro = Latin 7–10**
  calendar trick; *dez* also gives *dízimo* ("tithe," a tenth), twin of English
  *dime*.
- Taxonomy: namespaced `PT-NUM-1-5`, `PT-NUM-6-10`.

## Chapter 5 — The first verbs (completes the 5-language verb set)

- **Chapter 5 authored** (`PT-C05-falar`, `-morar`, `-trabalhar`,
  `-falo-portugues`, `-practice`): Portuguese's first **grammar-engine** chapter —
  and the **fifth and final** track to cross from fixed phrases into
  **self-assembled sentences**. Parallel to Spanish Ch.6 / French Ch.5 / German
  Ch.5 / Italian Ch.5.
- **The regular -ar present tense** — drop *-ar*, add *-o/-as/-a/-amos/-am*.
  Taught on **falar**, cemented on **morar** and **trabalhar**. Portuguese is
  **pro-drop** (drops *eu*, like Spanish and Italian).
- **Etymology, with the sharpest Iberian contrasts**:
  - *falar* ← *fabulārī* — the **same root as Spanish *hablar***, but **Portuguese
    kept the Latin *f-*** where Spanish shifted *f→h* (*falar/hablar*,
    *filho/hijo*, *fazer/hacer*) — the mirror of the Spanish Ch.6 f→h lesson.
  - *morar* ← *morārī* "to linger/tarry" (→ moratorium, demur) — a root **no
    sibling uses** for "dwell."
  - *trabalhar* ← *tripaliāre* "torture" — twin of Spanish *trabajar* (Latin *-li-*
    → PT *-lh-* / ES *-j-*); Portuguese rejoins the "torture" camp (ES/FR/PT) vs
    Italian's "labour" (*lavorare*).
  - First sentence: **Falo português** (*português* ← *Portus Cale*, a harbour town).
- Taxonomy: namespaced `PT-VERB-FALAR/MORAR/TRABALHAR`, `PT-WORD-PORTUGUES`.
- **Milestone**: all five tracks (ES/PT/IT/FR/DE) now build real sentences.

## Chapter 3 — Introducing Yourself

- **Chapter 3 authored** (`PT-C03-eu`, `-me-chamo`, `-como-se-chama`, `-prazer`,
  `-practice`): fills the gap between the greetings/how-are-you chapters and the
  farewells, so Portuguese runs greet → introduce → how-are-you → goodbye end to
  end. Each lesson reviews Chapter 2.
- **eu** (← *ego*) introduces **pro-drop** — the subject pronoun is usually
  dropped (shared with Spanish *yo* / Italian *io*).
- **me chamo / chamo-me** — "I call myself" (*chamar-se* ← Latin *clāmāre*),
  completing the three-way *cl-* sound split: Spanish *ll* (*llamo* = *y*),
  Italian *ch* (hard *k*), Portuguese *ch* (= *sh*). Notes the noun-route
  alternative *o meu nome é* (*nome* ← *nōmen* → noun/nominal).
- **Como se chama?** — the name asked with *como* ("how"), everyday *você*;
  **prazer** ← *placēre* "to please" (twin of Italian *piacere*; the Portuguese
  *pl-* → *pr-* shift).
- Uses canonical `PRONOUN-I`, `INTRO-MY-NAME-IS`, `INTRO-WHATS-YOUR-NAME`,
  `INTRO-NICE-TO-MEET-YOU` — no taxonomy change.

## Chapter 4 — Farewells (completes the 5-language greet-to-goodbye arc)

- **Chapter 4 authored** (`PT-C04-adeus`, `-ate-logo`, `-ate-amanha`,
  `-ate-breve`, `-practice`): closing a conversation, reviewing Chapter 2.
  Portuguese becomes the **fifth** track to reach the full greet → how-are-you →
  goodbye arc. Numbered Ch. 4 so introductions can slot in at Ch. 3.
- **adeus** ← *a Deus* "to God" — the exact twin of Spanish *adiós*, French
  *adieu*, Italian *addio*, and English *goodbye* ("God be with ye").
- **Reinforces the Arabic-loanword deep dive**: *até* ("until") ← **Arabic
  *ḥattā*** — the *same* borrowed function word as Spanish *hasta*, so *até logo*
  is the word-for-word twin of *hasta luego* (*logo* ← *locō*, kin of *luego*).
- **Cross-links**: *até amanhã* (*amanhã* ← *ad māneāna*, kin of *mañana*; the
  Portuguese **nh** = Spanish **ñ**); *até breve* (*breve* ← *brevis* "short" →
  brief/brevity/abbreviate).
- Uses the canonical `FAREWELL`, `FAREWELL-LATER`, `FAREWELL-TOMORROW`,
  `FAREWELL-SOON` concepts — no taxonomy change.

## Chapter 2 — "Tudo bem?" (the how-are-you chapter)

- **Chapter 2 authored** (`PT-C02-de-nada`, `-como`, `-tudo`, `-tudo-bem`,
  `-como-vai-esta`, `-mais-ou-menos`, `-practice`, `-formal-practice`): the
  "how are you?" exchange, atom-first,
  reviewing Chapter 1. Fifth and final track in the PR's cross-language
  how-are-you set, reusing `STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`,
  `WORD-SOSO`. Reordered ahead of introductions to widen the set (register
  você / o senhor introduced inline).
- **Portuguese's distinctive move — the verb-free greeting**: *Tudo bem?*
  ("everything well?") uses **no pronoun and no verb**, dodging the
  você/tu/senhor tangle; built on **tudo** ← Latin *tōtus* "whole" (→ total/
  teetotal) + **bem** ← *bene*. It also does *Como vai?* (*ir*, "to go" — with
  French/German) **and** *Como está?* (*estar*, "to stand" — with Spanish/
  Italian), so Portuguese spans all three patterns.
- **Etymology hooks**: *de nada* ← *nāta* "born thing" (exact twin of Spanish);
  *como* ← *quōmodo*; *mais ou menos* ← *magis* + *minus* (twin of Spanish *más
  o menos*); and *você* ← *vossa mercê* "your mercy" — the same "your grace"
  origin as Spanish *usted*.
- Taxonomy: namespaced `PT-WORD-TUDO` documented.

## Chapter 1 — Greetings (track bootstrapped)

- New Portuguese track on the HL00 framework: one word per lesson, slug ids,
  gender-before-nouns, atom-first, derivations shown, LaTeX book (Latin Modern;
  CI auto-discovers `portuguese/book/`).
- Chapter 1 (`lessons/PT-C01-*`), atom-first:
  - **olá** ("hi"; a rootless interjection, twin of Spanish *hola*).
  - **bom / boa** ("good" ← *bonus*; nasal *bõ*; adjective agreement).
  - **o / a** ("the"; gender ← *ille/illa*, eroded to a single vowel — further
    than Italian *il* / Spanish *el*).
  - **dia** ("day" ← *dies*; the gender trap — masculine despite *-a*).
  - **bom dia** (assembled; singular vs. Spanish plural *buenos días*).
  - **tarde / boa tarde** ("afternoon" ← *tardus*, "late"; English *tardy*).
  - **noite / boa noite** ("night" ← *noctem*; Latin *-ct-* → *-it-*, like
    French *nuit*).
  - **obrigado / obrigada** ("thanks" ← *obligātus*, "obliged"; English
    *obligated*) — agrees with the **speaker**, not the noun.
  - **practice**.
- Grounds each word against English + Latin, with Spanish/French/Italian
  supplied for contrast (beginner-audience). Book compiles clean with XeLaTeX.
