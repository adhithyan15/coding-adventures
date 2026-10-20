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

