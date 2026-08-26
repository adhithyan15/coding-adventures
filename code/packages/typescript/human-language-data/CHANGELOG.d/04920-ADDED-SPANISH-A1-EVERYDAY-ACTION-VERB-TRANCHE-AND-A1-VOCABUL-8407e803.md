### Added - Spanish A1 everyday-action verb tranche, and A1 vocabulary reaches 601

- Author sixteen verb lessons across chapters 381-384, sequences 7970-8120, one
  headword and one new atom per lesson, all on the A1
  `SPINE-NAME-EVERYDAY-ACTIONS` node added in the previous change. Spanish moves
  **585 -> 601** distinct headwords at or below A1, crossing the `HL09` §3.1
  floor of 600, and **8 -> 24** distinct verb headwords.
- **The two numbers now disagree, which is the point.** Criterion 2 is satisfied
  at 601 of 600; criterion 2b fails at 24 verbs against 40. A gate carrying only
  the count would wave A1's vocabulary through from here. `level-gate.test.ts`
  asserts exactly that — the vocabulary blocker is now *absent* while
  `verb-vocabulary(-16)` remains — so the composition criterion is doing visible
  work rather than agreeing with its neighbour.
- **At or below pre-A1 stays at 304** against the floor of 300. All four
  headwords of slack survive and Spanish keeps `attained: pre-A1`, the only level
  any track holds. Every lesson in the tranche is A1 by construction, so the
  pre-A1 count cannot move.
- Ship all sixteen **namespaced** (`ES-VERB-LAVAR` and friends). None of the 46
  canonical `VERB-*` concepts names *to wash*, *to climb*, *to look for*, *to
  keep*, *to die* or *to send*, and promoting them would ask all 23 tracks to
  answer for them. `verbs.test.ts` records the consequence honestly: Spanish's
  `extras` moves **10 -> 26**, which reads as "sixteen everyday verbs the
  cross-language core does not name yet" — a question for the taxonomy, not a
  defect in the track.
- Screen every candidate three ways — headword list, **atom** ledger, **root**
  ledger — plus the initial-stem rule. The screen dropped `cerrar` as an outright
  headword duplicate and `llevar`, `entrar`, `bajar`, `llegar`, `cambiar`,
  `limpiar`, `viajar`, `cocinar`, `bailar`, `nacer` and `coger` on a spent root
  or a shared opening stem. `aprender` was dropped despite screening clean:
  `VERB-LEARN` is a canonical concept owned by the A2 node, and a namespaced
  duplicate beside it is the drift `verbs.ts` exists to measure.
- **The screen's own self-test caught the screen.** The first version read
  `introduces.knowledge` as a nested object; `parse.ts` flattens frontmatter to
  dotted keys, so the atom ledger loaded **zero** atoms and would have certified
  any candidate at all. A two-directional assertion — a known atom must be
  present, an impossible one must be absent — is what failed, and it now ships
  with the screen.
- **Etymology: two hooks were rewritten after the sources contradicted the plan.**
  `sacar` was going to teach that *sack* is a false friend. That overcorrection is
  as wrong as the naive claim: the denominal route really is dead, because a verb
  from *saco* would mean putting *into* a sack, but Latin *saccus* also named a
  **filter** and *saccāre* is attested only meaning to filter. The lesson now
  teaches the settled negative and leaves the fork open. `buscar` was going to
  hedge toward Corominas's Celtic proposal; his actual headword verdict is **de
  origen desconocido**, he calls the hypothesis "muy arriesgada", and no
  Proto-Celtic dictionary lists *buscar* among that root's descendants. The
  lesson teaches the dead end, refutes the *bosque* story on chronology
  (*bosque* arrives four centuries after *buscar* is attested), and hooks on
  English *busker*, which needs no origin at all.
- Six further hooks were circular as planned — the gloss restating the ancestor
  and handing it back as proof — and each was rescued with a cognate doing real
  work: `lavar` (lavatory, launder, lavish), `subir` (*sudden* < *subitus*, and
  the paradox of a verb meaning "go up" that begins with *sub-*), `morir`
  (*mortgage*, a dead pledge), `romper` (*route* < *rupta via*), `saltar`
  (*somersault* = *supra saltus*), `soñar` (*insomnia*). `morir` also states
  explicitly that *murder* is a **Germanic** cousin sharing only the deeper
  Indo-European root — "same Latin root" would have been false.
- Stop where the evidence stops. `olvidar` teaches *oblivion* and goes no
  further, because the *ob-* + *lēvis* analysis is rejected by specialists;
  `lavar` refuses *lava* and *lavender*, both disputed. False friends taught
  explicitly: `pagar`≠*pagan*, `enviar`≠*envy*, `soñar`≠*sonar*, `saltar`≠*salt*,
  `romper`≠*romp*, `morir`≠*morgue*.
- **The glyph gate earned its keep.** Two characters the Spanish book has never
  rendered slipped into the prose — `À` in *à chef* and `ð` in Old English
  *morðor*. Both were removed at source rather than added to a repertoire nobody
  has proved: the first re-cased, the second transliterated as *morthor* with a
  note that English has since lost the letter.
- Hold every content pin. `ruleStatements`, `paradigmTables`, `fullParadigmGrids`
  and `lessonsWithFindings` unchanged; banned words absent from all sixteen
  lessons; Spanish cross-chapter prose references still zero. **No pin was
  lowered and no ceiling raised.** The two pins that moved are floors that grew
  with the corpus, each with a comment saying why. The derived bundle gate
  reports **286 lesson batches over 285 chapter bands** — both sides moved by
  two together, which is the property that change was made to have —
  `BAND_SPLIT_SLACK` untouched at 1 and the 256 kB backstop untouched.
