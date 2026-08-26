# A1 everyday-action verbs — the first tranche on the new node

Sixteen verbs across chapters 381–384, sequences 7970–8120. Spanish crosses the
A1 vocabulary floor: **585 → 601** distinct headwords at or below A1, against
the HL09 §3.1 target of 600. Verb headwords at or below A1 go **8 → 24**.

At or below pre-A1 stays at **304** against a floor of 300. Every lesson here is
A1, so the four headwords of slack are untouched and Spanish keeps the only
attained level in the project.

| Ch. | Spine node | Words |
|---|---|---|
| 381 | `SPINE-NAME-EVERYDAY-ACTIONS` | lavar, romper, saltar, soñar |
| 382 | `SPINE-NAME-EVERYDAY-ACTIONS` | empezar, acabar, pagar, quedar |
| 383 | `SPINE-NAME-EVERYDAY-ACTIONS` | guardar, dejar, sacar, buscar |
| 384 | `SPINE-NAME-EVERYDAY-ACTIONS` | enviar, subir, morir, olvidar |

## The duplicate hunt

Three screens — the headword list, the atom ledger across all lesson types, and
the root ledger — plus the initial-stem rule.

| Dropped | Screen | Why |
|---|---|---|
| `cerrar` | headword | already taught; root `serare` spent |
| `llevar`, `bajar`, `coger`, `nacer` | root | `levare`, `bassus`, `colligere`, `nasci` all spent |
| `entrar`, `llegar`, `cambiar`, `viajar`, `bailar` | root + stem | `la entrada`, `la llegada`, `el cambio`, `el viaje`, `el baile` |
| `limpiar`, `cortar`, `cocinar`, `descansar`, `necesitar` | stem | `limpio`, `corto`, `el cocinero`, `el descanso`, `necesario` |
| `mirar`, `ganar`, `nadar`, `cantar` | stem | `mira`, `el ganado`, `nada`, `la cantidad` |
| `aprender` | taxonomy | `VERB-LEARN` is canonical and owned by the A2 node; a namespaced duplicate is drift |

The screen very nearly certified everything. Its first version read
`introduces.knowledge` as a nested key, but `parse.ts` flattens frontmatter to
dotted keys, so the atom ledger loaded **zero atoms** and every candidate came
back clean. The two-directional self-test is what caught it — a known atom must
be present, an impossible one must be absent — and a one-directional "did I load
anything?" check would have passed cheerfully.

## Etymology

Two hooks were rewritten after the sources contradicted the plan, and both
corrections were toward *less* certainty rather than more.

**`sacar`.** The plan was to teach *sack* as a false friend. That is an
overcorrection, and as wrong as the naive claim. The denominal route is dead —
a verb built on *saco* would mean putting *into* a sack — but Latin *saccus*
also named a **filter**, and *saccāre* is written down only in the sense of
filtering wine and water. So the bag may be a distant relative after all, by way
of a strainer. The lesson teaches the settled negative and leaves the fork open
between Corominas's Gothic *sakan* and the Romanist *saccāre*.

**`buscar`.** Corominas's headword verdict is **de origen desconocido** — his
strongest negative, not merely "uncertain". He raises the Celtic idea as a
question and then calls it *muy arriesgada*, and no dictionary of reconstructed
Celtic lists *buscar* among that root's descendants. The dictionary line that
looks authoritative is a promotion of a hypothesis its own author disavowed. The
lesson teaches the dead end, refuses the *bosque* story on chronology, and hooks
on English *busker*, which needs no origin.

Six hooks were **circular** as planned — the gloss restating the ancestor and
offering it back as proof — and each was rescued with a cognate doing real work:
`lavar` → lavatory/launder/lavish, `subir` → *sudden*, `morir` → *mortgage*,
`romper` → *route*, `saltar` → *somersault*, `soñar` → *insomnia*.

`morir` states explicitly that **murder** is a Germanic cousin sharing only the
deeper Indo-European root. "Same Latin root" would have been false.

Stopped deliberately: `olvidar` teaches *oblivion* and no further, because the
*ob-* + *lēvis* analysis is rejected; `lavar` refuses *lava* and *lavender*.

Two characters had to come out of the prose because no Spanish book has rendered
them — `À` in *à chef* and `ð` in *morðor*. Re-cased and transliterated at
source rather than added to an unproved repertoire.

## Pins

No pin was raised and no ceiling touched. `ruleStatements`, `paradigmTables`,
`fullParadigmGrids` and `lessonsWithFindings` unchanged; banned words absent;
cross-chapter references still zero. The bundle gate reports 286 batches over
285 bands — both sides moved together.

Two floors grew with the corpus, each with a comment: the level-gate pin (the
vocabulary blocker is now *gone* and `verb-vocabulary(-16)` is what remains) and
`verbs.test.ts`'s `extras` count, 10 → 26, recording that sixteen everyday verbs
have no canonical concept yet.
