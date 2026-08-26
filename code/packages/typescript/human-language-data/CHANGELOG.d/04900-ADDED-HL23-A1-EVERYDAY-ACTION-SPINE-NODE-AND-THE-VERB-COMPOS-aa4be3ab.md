### Added - HL23 A1 everyday-action spine node and the verb composition criterion

- Add `SPINE-NAME-EVERYDAY-ACTIONS`, stage **A1**, strand **LEXICON**, `canDo`
  *"I can name common everyday actions."* — the first slice of `HL09` §11 item
  5. It carries three concepts split off `SPINE-SAY-WHAT-I-DO`: `VERB-DRINK`,
  `VERB-GIVE`, `VERB-PUT`. That node goes **42 -> 39** concepts and stays the
  spine's worst `over-ceiling` offender, which is why the pin moved rather than
  disappeared.
- Add criterion **2b** to the level gate: `verbVocabularyOf` counts distinct
  headwords whose `concept_tag` names a verb, against a new
  `LEVEL_VERB_VOCABULARY` floor. Criterion 2 counted headwords and asked nothing
  about part of speech, which is how Spanish reached **584 of 600** at or below
  A1 with **seven** verb-tagged headwords and the complete present paradigm of
  all three conjugations. The learner had the machinery and nothing to run it
  on. The gate now says so: Spanish is blocked at A1 on `verb-vocabulary(-32)`
  **and** `vocabulary(-15)` — two numbers from two criteria, which is the whole
  argument for asserting composition beside a count.
- Record the floors as a **project choice**, not a derived one. No awarding body
  publishes a verb quota and none is cited. The doc comment states the shape
  instead — 1.7% of the total at pre-A1, 6.7% at A1, 10% from A2 up — and the
  direction of error: `concept_tag` has 96.2% recall on verbs, the residual 4%
  are verbs the tag misses, so the count runs **low** and the criterion flags a
  track far more readily than it certifies one.
- **Correct `HL23`'s own costing of the option it recommended**, in a new §8.
  The spec priced Option C at 46 ledger entries. It missed
  `unclassified-curriculum-extension-lesson`: a lesson in a path segment is
  either the canonical realization of one of that node's concepts or it is local
  support and must sit in an extension, so moving a concept **reclassifies every
  lesson realizing it, in every track**. Only five of the 42 concepts were
  releasable without editing another track. The recommended six-concept set
  costs **19 lesson migrations across 13 tracks**, and `VERB-LIVE` additionally
  fails `misplaced-shared-realization` on `FR-C05-habiter` and `GE-C05-wohnen`,
  which carry no explicit `spine_node`. §8.2 prices every concept individually
  so the continuation is planned off measurement rather than off the old table.
- Move exactly one lesson, and only inside Spanish. `ES-PATH-029` held two
  lessons, so `ES-C07-beber` became its own segment `ES-PATH-A1-BEBER` under the
  new node. Spanish goes **584 -> 585** headwords at or below A1 and **7 -> 8**
  verb headwords; **at or below pre-A1 stays at 304** against the floor of 300,
  so all four headwords of slack survive and the only level any track holds is
  untouched.
- Relocate **none** of the 26 misfiled A1 grammar lessons. The plural present
  paradigm under `SPINE-ASK-LOCATION` and the gerunds, imperfect, preterite,
  agreement and infinitive-as-subject under `SPINE-DEFINITE-REFERENCE` are
  morphology; this node's `canDo` claims only naming. Widening it to cover both
  is the compound capability statement `HL23` §5 rejected Option B for. They
  need a GRAMMAR rung of their own.
- Derive all 46 ledger entries from `validateCurriculum`'s own rules rather than
  hand-writing them — `omits`, `relocates` and `segments` are compared with
  `JSON.stringify`, so element and key order are part of the contract. Ledgers
  and monoliths are regenerated with `--unshard`/`--shard`, never hand-merged.
- Hold every ceiling. The derived bundle gate reports **284 lesson batches over
  283 chapter bands**, unchanged; `BAND_SPLIT_SLACK` stays 1 and the 256 kB
  backstop is untouched. Two pinned corpus counts moved with comments saying why
  — `totalNodes` 33 -> 34, `largestNode` 42 -> 39 — and no assertion was loosened.
