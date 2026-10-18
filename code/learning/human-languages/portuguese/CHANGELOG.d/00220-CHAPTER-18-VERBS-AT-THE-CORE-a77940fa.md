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

