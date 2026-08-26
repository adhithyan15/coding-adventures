### Added - Spanish A1 everyday-action verbs, tranche two, closing criterion 2b

- Author sixteen verb lessons across chapters 385-388, sequences 8130-8280, one
  headword and one new atom per lesson, all on the A1
  `SPINE-NAME-EVERYDAY-ACTIONS` node: `usar`, `tocar`, `mezclar`, `colgar`,
  `vender`, `servir`, `gastar`, `crecer`, `quemar`, `cruzar`, `volar`, `girar`,
  `empujar`, `lanzar`, `borrar`, `gritar`. Verb headwords at or below A1 go
  **24 -> 40** against a floor of 40, and distinct headwords **601 -> 617**.
- **The `verb-vocabulary` blocker is discharged.** Both A1 vocabulary criteria
  are now satisfied for Spanish. A1 is still not attained and what holds it is
  `reinforcement(-83)` -- a different criterion, reported explicitly by
  `level-gate.test.ts` so that "the verb blocker is gone" cannot be misread as
  "A1 is done".
- **Closing the criterion removed its only red instance, so a synthetic one
  replaces it.** A fixture track meets criterion 2 exactly -- every
  `LEVEL_VOCABULARY["pre-A1"]` headword, all distinct -- and carries one verb
  fewer than criterion 2b asks for; the assertion is that `verb-vocabulary(-1)`
  is its sole blocker, with a counterfactual showing that retagging that one
  lesson attains the level. Without it `verbVocabularyOf` could have been deleted
  and the suite would still have passed.
- **At or below pre-A1 stays at 304** against the floor of 300. Every lesson is
  A1 by construction, so the four headwords of slack cannot move and Spanish
  keeps `attained: pre-A1`, the only level any track holds.
- Ship all sixteen **namespaced**. No canonical `VERB-*` concept names to use, to
  touch, to sell, to serve, to spend, to grow, to burn, to cross, to hang, to
  mix, to turn, to fly, to push, to throw, to erase or to shout, and minting
  sixteen would ask all 23 tracks to answer for them. `verbs.test.ts` records the
  consequence: Spanish's `extras` moves **26 -> 42**, now larger than `covered`,
  which is the point at which "the core does not name these" stops being a
  rhetorical question.
- **The duplicate screen grew a fourth ledger, because the third has holes.**
  Batch one screened headwords, atoms and roots plus the initial-stem rule, and
  that set passes `volver` and `doler` as clean -- both already taught.
  `ES-C288-vuelta` declares `roots: []` and then spends *volvere*,
  *volume*-as-a-rolled-scroll and *revolve* in its prose; `ES-C286-dolor`
  declares `roots: []` and spends *dolere*, *condolence* and *indolent*. The
  root ledger only knows what a lesson chose to declare. The screen now indexes
  gloss, hook and body and matches proposed etymons against the text, which also
  dropped `llorar` (`ES-C316-llanto` tells *plorare*), `mover`
  (`ES-C303-momento`), `pintar` (`ES-C376-pimiento`), `firmar`
  (`ES-C304-enfermo`), `curar` (`ES-C284-seguro`) and `dibujar`
  (`ES-C291-bosque`) -- none visible to any other screen.
- Every ledger carries a **two-directional self-test**, and the headword one
  fired on the first run: its anchor was `el suelo`, which Spanish does not
  teach. Loaded totals are reported rather than assumed -- 948 headword tokens,
  1,249 introduced atoms, 649 declared roots, 982 prose bodies.
- **Etymology, three passes, and the third moved the most.** `quemar` was drafted
  as settled Latin `cremare` and ships as an open argument: DLE prints one
  unhedged line, Corominas opens by saying phonetics **forbid** it -- Portuguese
  *queimar* needs an older *ai*, and losing *cremare*'s *r* is *gravisimo* -- and
  proposes a remaking under a Greek word for a burn while conceding his own soft
  spot. Root recorded as `quemar-disputed`.
- `cruzar` is **not** from `cruciare`: Corominas knows *cruciare* already meant
  "to cross" by c. AD 500 and rejects it on phonetic grounds, so `cruzar` is
  denominal on *cruz* -- and *cruz* is semi-learned, since short Latin *u* should
  have given *croz* and the surviving *u* is the fingerprint *Dios* also carries.
  Root corrected `cruciare-latin` -> `crux-latin`.
- `lanzar` is the `buscar` pattern: the verb is settled, but DLE calls *lancea*
  Celtiberian outright where Corominas says only *quizá* and recent specialists
  argue a different ancient Iberian language. `usar` carries a **star** -- DLE
  prints no etymology at all, and the ninth-century legal-Latin attestations are
  too late to document the stage that matters, so neither "attested" nor
  "unattested anywhere" is true. `gritar`'s Quirites story is taught as **folk
  etymology**, labelled, because Corominas suspects the ancient lexicographers
  built the gloss out of the resemblance. `empujar` hedges twice in both
  dictionaries, and English **push** is from plain *pulsare* without the *in-*,
  making it a cousin rather than the same word.
- `vender` gained a morphological proof of the *venum dare* contraction: the
  perfect **vendidi** and participle **venditum** still show *dedi* and *datum*
  inside them, and the long *e* before *-nd-* is anomalous unless two words
  merged. Further corrections from sources rather than summaries: *waste* is
  **West Germanic** influence on *vastare*, not Frankish; *vast* dropped, since
  Latin *vastus* probably merges two words; *cremate* is an 1851 back-formation
  from *cremation*; *volatilis* meant *winged*, and the English senses arrive in
  the reverse order; *bureau*'s cloth is an unresolved either/or, so `borrar`
  teaches the desk and names the doubt; the `gyros` you eat reached English
  straight from modern Greek in 1971 and **never through Latin**, the mirror of
  the *murder*/*morir* trap; and `servus`'s own origin is left open between
  "guardian" and an Etruscan loan.
- **No ceiling raised.** `ruleStatements`, `paradigmTables`, `fullParadigmGrids`,
  `lessonsWithFindings` and the banned-word ceiling all unchanged -- one *just*
  reached `servir` and the **sentence** was rewritten, not the number.
  `ES-C387-cruzar` first measured **325 effective seconds** against the
  300-second model and was **cut**, not re-declared. The glyph gate is clean and
  the four new chapters introduce no character the Spanish book was not already
  rendering. All nine generated-artifact checks pass and measurements were taken
  from a from-scratch `dist/`.
