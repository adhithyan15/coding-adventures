### Spanish A1 vocabulary tranche 7 — thirty-five exam-derived lexemes (chapters 390-396)

The first of three authoring tranches on the run to `APTO`, and the first tranche in the series
whose word list was chosen by measurement rather than by topic. Every one of the thirty-five is a
lexeme a mock DELE A1 item actually requires.

Five noun chapters land on `SPINE-NAME-EVERYDAY-THINGS`, two verb chapters on
`SPINE-NAME-EVERYDAY-ACTIONS`. Neither node's `canDo` changed, because neither had to: the rungs
`HL23` §10 and §11 built are exactly the rungs this vocabulary needs.

```
  390  THINGS   la terraza, la sala, la planta, el ascensor, la piscina
  391  THINGS   el menú, el camarero, el postre, el supermercado, el euro
  392  THINGS   el aeropuerto, las vacaciones, el destino, el pasajero, agosto
  393  THINGS   el ordenador, internet, el correo electrónico, la dirección, la nacionalidad
  394  THINGS   la universidad, el trabajo, la actividad, la guitarra, el médico
  395  ACTIONS  entrar, volver, llegar, perder, necesitar
  396  ACTIONS  aprender, cocinar, preferir, funcionar, reparar
```

Sequences 8300-8640, one headword and one new atom per lesson. Seven chapters of five, seven
`chapters.d`, seven `curriculum.d/path`, seven `curriculum.d/extensions`, seven
`core/book-generation.d` targets, and one appended `segments` line in each of the two A1 spine
ledgers. The three monoliths are regenerated with `--unshard`, never hand-edited. Shard integrity
is asserted on PARSED COUNTS rather than bytes: 396 chapter shards against 396 monolith sections,
413 path, 391 extensions, 38 spine.

No canonical concept was minted and no other track was touched. The thirty-five lessons carry
namespaced `ES-THING-*` and `ES-VERB-*` tags and ride in per-chapter extensions, so
`conceptOwner` stays undefined and the `unclassified-curriculum-extension-lesson` branch of
`HL23` §8.1 never fires. That is why a tranche this size costs zero foreign ledger entries.

#### The bundle proof came first, and it moved the plan twice

Before a word was written, the calibrated harness re-sat both mocks and scored every bundle. The
baseline reproduces `HL23` §11.5 exactly — mock 1 Grupo 1 **12,33**, mock 2 **7,17**, 78 objective
items failed — which is what licenses the projections below.

Granting all 103 candidate lexemes returns **31,33** and **32,33**, `APTO` on both. §11's table
predicted 33,33 and 33,33; the gap is the three staged entries #13154 deliberately left behind
(`cansado`, `estudiante`, `lado`), and granting those three as well returns **32,33** and
**33,33** — mock 2 exactly on the published figure, mock 1 one objective item short, which is the
documented residual `HL23` §10.6 records on mock 1 *lectura*.

**The proof then refuted the plan twice, which is the whole reason it is run first.**

The screen dropped ten of the 103 on the shared-initial-stem rule, and dropping all ten returns
**30,33 / 28,33** — `NO APTO` on mock 2. Separately, the pure adjectives among the 103 have no
honest A1 rung to stand on, and dropping those returns **30,33 / 26,33** — `NO APTO` again. Both
findings are recorded in `BACKLOG.d`, because both are owner decisions rather than authoring work.

This tranche therefore ships the largest **honest** slice available: thirty-five nouns and verbs
that are clean on all three duplicate ledgers and on the confusability screen, and that need no
new spine node and no widened `canDo`.

#### The screen earned its place again, and found a fifth miss-mode

Screening ran against headwords across every lesson type, introduced atom ids, and the root
ledger. It reconfirmed the four already-owned traps a headword-only screen misses — `llevar`
(`ES-C39-traer`), `andar` (`ES-C36-caminar`), `dar` (`ES-C65-di`, a `grammar` lesson) and `llover`
(`ES-C30-llueve`, a `phrase`).

It also found a new one. **`amigo` is owned by `ES-C09-falsos-amigos`**, whose headword is the
two-word term of art *falsos amigos*, at A2. No headword, atom or root screen sees it; only a
screen that decomposes multiword headwords into component words does. It is also the case where
"already owned" is arguably wrong, since the corpus teaches the metalinguistic term and not the
word *friend* — which is why it is filed as a decision and not silently authored.

One correction to the screen itself is worth recording. Duplicate detection and confusability
detection need **different indexes**, and conflating them produces false drops: `mejor` lifted out
of the idiom *pasar a mejor vida* is a fragment, never presented to a learner as a word, so it
cannot be the thing a learner confuses `menor` with. Duplicates use the wide index (compounds,
articles, plurals, `+` patterns, ellipsis); confusability uses whole displayed headword forms only.

#### Etymology was verified before any prose was written, in a dedicated pass

Five independent verification passes covered all thirty-five words before a single lesson was
drafted. `dle.rae.es` 403s plain fetchers throughout; `josecanovas.com` was excluded; no
search-engine summary was used as evidence. The pass killed or rewrote claims that would otherwise
have shipped:

- **`aeropuerto` is not a calque of English *airport*.** No source says so, and CNRTL derives
  French `aéroport` (1922) as a French internal compound. The structural-identity argument for a
  calque is circular — independent `aero-`+`port` compounding predicts the same shape.
- **`llegar` does not come from `applicāre` "per Corominas".** Both Wiktionaries print that etymon
  and footnote DCECH for it; **Corominas gives `plicāre`**, back-formed from `applicāre`. The
  semantic bridge is the attested nautical idiom `applicāre nāvem`, not sails being folded — that
  story appears in no source and smuggles its conclusion into its premise.
- **`euro` has a homonym trap in DLE's own pages**: `euro1` is a Greek-derived name for a *wind*.
  A lesson reaching for "from Greek" teaches the wind. The currency is a clipping of `europeo`,
  and the Pirlot coinage story is an attributed claim, not a documented fact.
- **`universidad` has nothing to do with a universe of subjects.** `universitas` was a legal term
  for a corporation or guild; the circular "all knowledge" gloss is refused, and the 1490 Spanish
  attestation meaning "totality" is used instead.
- **`necesse` is not settled as `ne-` + `cessus`.** Baldi (2013) shows the `cessus` form postdates
  the word by centuries. `perder` likewise does not get `per-` + `dare` printed as fact, since the
  second element is disputed between `dare` "give" and a separate root meaning "put".
- **`reparar`'s route is genuinely undecided**, and the reason is the teachable point: no segment
  in `re-par-ar` undergoes any change that would differ between the inherited and learned routes,
  so phonology cannot decide it.
- **`guitarra`'s Arabic step is contested**, not settled: DLE adds an Aramaic stage Corominas does
  not have, and etymonline says the direction of borrowing may be the reverse.
- **`trabajo`'s *tripalium* is real** — attested as `trepalium` in the Council of Auxerre (578) —
  but the *verb* `*tripaliāre` is reconstructed, and the soft joint is `tripalium` from `tripalis`,
  not the descent of `trabajo`. The moralized "work is torture" gloss is not an etymological finding.

#### No pin was raised

`ruleStatements` stays at its ceiling of 30 — every sound change in the tranche is shown as an
instance rather than stated as a rule, which is why `planta`, `dirección`, `agosto`, `volver` and
`preferir` print their evidence and never their law. `≤pre-A1` stays at **304**, untouched: every
lesson here is A1. Banned words unchanged, no novel glyphs, `standalone-book` clean, and no lesson
exceeds 240 declared seconds.
