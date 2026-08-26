# HL23 — The A1 verb gap

**Status:** decision document. Nothing here is implemented, and nothing here
should be implemented until the owner has chosen an option. The spine, the track
ledgers and the lesson corpus are untouched by the commit that carries this file.

**Tracking:** #12984. **Measured against:** `origin/main` @ `3c442dcf16`
(Spanish A1 vocabulary tranche 6).

---

## 1. Why this document exists

Spanish is 16 distinct headwords short of the 600 that `HL09` §3.1 asks for at or
below A1. One more vocabulary tranche closes that number. The concern raised was
that the number would then certify A1 for a vocabulary containing essentially no
verbs, and that after certification the defect would stop being visible in any
measurement the project takes.

The concern is substantially correct, but not in the shape it was reported. This
document establishes what is actually true by measurement, diagnoses precisely
what is in the way, prices the ways out, recommends one, and sketches the gate
that would have caught the class.

---

## 2. The measurement

### 2.1 The counting rule, and why the replication is evidence

`vocabularyOf` in `level-gate.ts` is the whole of criterion 2:

```ts
function vocabularyOf(lessons: ParsedLesson[]): number {
  const words = new Set<string>();
  for (const lesson of lessons) {
    if (!CONTENT_TYPES.has(lesson.realization.type)) continue;
    const headword = (lesson.realization.headword ?? "").trim().toLowerCase();
    if (headword) words.add(headword);
  }
  return words.size;
}
```

`CONTENT_TYPES` is `{"word", "phrase"}`. A lesson's level is its spine node's
`stage`; "at or below A1" spans the seven `pre-A1` nodes and the four `A1` nodes,
because `CEFR_LEVELS` begins `["pre-A1", "A1", …]` and `pre-A1` therefore ranks
below `A1` rather than outside the ladder.

The figures below come from a re-implementation of that rule written from the
source rather than by calling it. **The replication reproduces the gate's 584
exactly**, which is why the distribution it reports can be trusted: the same
traversal that yields the official number yields these parts.

### 2.2 The distribution

| Spanish, at or below A1 | count |
|---|---|
| content lessons (`word`/`phrase`) | 585 |
| **distinct headwords** | **584** (target 600) |
| distinct headwords carrying a `VERB` concept tag | **7 (1.2%)** |
| distinct verb **lexemes** | **5** |

The seven, in full — this is the entire verbal lexicon a Spanish learner owns at
A1:

| headword | gloss | tag | hosting node | stage |
|---|---|---|---|---|
| `estar` | to be (the standing kind) | `ES-VERB-ESTAR` | `SPINE-CHECK-WELLBEING` | pre-A1 |
| `estudiar` | to study | `ES-VERB-ESTUDIAR` | `SPINE-POLITE-REQUEST-REPAIR` | pre-A1 |
| `hablar` | to speak / to talk | `VERB-SPEAK` | `SPINE-POLITE-REQUEST-REPAIR` | pre-A1 |
| `trabajar` | to work | `VERB-WORK` | `SPINE-POLITE-REQUEST-REPAIR` | pre-A1 |
| `ir` | to go | `VERB-GO` | `SPINE-ASK-LOCATION` | **A1** |
| `llamo` | I call (from *llamar*) | `ES-VERB-LLAMAR` | `SPINE-EXCHANGE-NAMES` | pre-A1 |
| `estás / está` | forms of *estar* | `ES-VERB-ESTAR-FORMS` | `SPINE-CHECK-WELLBEING` | pre-A1 |

Two of the seven are inflected forms of verbs already in the list, so the count
of distinct verb lexemes is five: *estar, estudiar, hablar, trabajar, ir*.

The rest of the 584 is things and their properties. By concept-tag family the
largest groups are `REF` (75), `WHERE` (60), `COUNT` (40), `CLOCK` (30), `BODY`
(27), `ASK` (25), `KIN` (20).

### 2.3 The premise about the A1 nodes is refuted as stated

The claim under review was that the A1 spine nodes "cannot host a verb". They
demonstrably can, and one already does: **`ir` is hosted on `SPINE-ASK-LOCATION`,
an A1 node**, and has been all along. What is true is narrower and worse — see §3.

### 2.4 What `vocabularyOf` cannot see, and this is the important part

The instruction was to stop if verbs are present through channels the count
misses. They are, in quantity, and it changes the severity assessment.

At or below A1, Spanish also carries **136 lessons `vocabularyOf` never counts**:

| type | count at or below A1 | counted by `vocabularyOf`? |
|---|---|---|
| `word` | 559 | yes |
| `grammar` | 55 | **no** |
| `practice-mix` | 38 | **no** |
| `phrase` | 26 | yes |
| `review` | 22 | **no** |
| `writing` | 12 | **no** |
| `etymology` | 9 | **no** |
| `practice` | 2 | **no** |

**32 of those uncounted lessons are verbal**, and every one is a `core` path
lesson rather than an extension — so they are gate-visible as *realizations* while
being invisible to the *vocabulary* criterion. They teach:

- the complete present paradigm of all three regular conjugations, singular and
  plural (`hablo`/`hablas`/`habla`/`hablamos`/`habláis`/`hablan`, and the same
  for `-er`/`-ir`);
- the present of `ir` (`vamos · vais`, `van`);
- gerunds (`hablando`, `comiendo · viviendo`);
- subject-verb agreement as an explicit rule;
- object pronouns with verbs (`lo tengo / le hablo`);
- the infinitive as subject (`comer es bueno`);
- and, anomalously for this level, imperfect and preterite forms
  (`hablábamos`, `hablasteis`, `di`).

A further **6 of the 26 `phrase` lessons embed a finite verb** — `Hablo español`,
`soy de…`, `¿Cuántos años tienes?`.

### 2.5 Honest severity: the machinery is taught, the lexicon is not

The backlog entry says such a learner "has reached a picture dictionary" and
"cannot say *I need*, *I learn*, *I break*, *I wash*". **That is overstated.** A
Spanish learner at A1 can inflect any regular verb of any of the three families
in the present, in any person, and can produce whole sentences with them.

The accurate statement is the inverse and is still a real defect: **the learner
has complete verbal machinery and almost nothing to run it on.** Five lexemes
is not a vocabulary; it is a paradigm table with three worked examples. Asked to
say *I wash my hands* or *I'm looking for the station* — both squarely inside the
A1 descriptors about immediate needs — the learner has the grammar and lacks the
word.

So: the gap is real, it is worth fixing, and it is **narrower and cheaper than
reported**. It is a lexicon gap, not a capability gap. Nobody needs to design a
verb pedagogy; the pedagogy is built. Somebody needs a node to hang words on.

### 2.6 It is systemic, but Spanish is the only track that can trip it

All twenty-three registered tracks:

| track | headwords ≤A1 | verb-tagged | | track | headwords ≤A1 | verb-tagged |
|---|---|---|---|---|---|---|
| **spanish** | **584** | **7** | | bengali | 47 | 1 |
| telugu | 222 | 5 | | chinese | 46 | **0** |
| sanskrit | 210 | **0** | | japanese | 44 | **0** |
| hindi | 192 | 2 | | persian | 43 | **0** |
| malayalam | 191 | 5 | | urdu | 43 | **0** |
| tamil | 191 | 4 | | punjabi | 39 | 2 |
| kannada | 188 | 5 | | russian | 39 | **0** |
| french | 70 | 1 | | marathi | 37 | 1 |
| arabic | 66 | **0** | | gujarati | 35 | 1 |
| latin | 66 | **0** | | marwadi | (unsharded) | **0** |
| german | 65 | 1 | | | | |
| portuguese | 65 | 5 | | | | |
| italian | 64 | 4 | | | | |

**Nine of twenty-three tracks have zero.** No track exceeds seven. (Marwadi
keeps an unsharded `curriculum.json` rather than a `curriculum.d/path`
directory, so its at-or-below-A1 slice was not computed by the same traversal;
it carries no verb-tagged content lesson at any level, so its zero is certain
even though its headword total is not stated here.)

Ground truth from running `runLevelGate` over the corpus: **Spanish is the only
track that has attained any level at all** (`pre-A1`). Every other track is *in
progress at pre-A1*, short on vocabulary by between 114 and 266 headwords.

### 2.7 A1 is further away than "one more tranche"

Spanish's A1 blockers are **`vocabulary(-16)` and `reinforcement(-83)`**. The
reinforcement criterion (§3.1 item 4 — every atom revisited at least twice) is 83
atoms short. A vocabulary tranche closes the first and not the second, so the
gate will not certify A1 on the next tranche. **There is more runway for this
decision than the framing assumed**, and the decision does not have to be rushed
to beat a certification.

---

## 3. The diagnosis

The instruction was that "the taxonomy" is not a diagnosis. It is not, and it is
also not the answer.

### 3.1 Nothing mechanically forbids a verb at A1

`validate.ts` asks only that a content lesson carry a tag which is canonical or
namespaced:

```ts
err("missing-concept", `${id}: content lesson has no concept_tag`, id);
…
} else if (!hasOwn(taxonomy.concepts, r.concept) && !NAMESPACED_TAG.test(r.concept)) {
  err("unknown-concept", `${id}: concept_tag '${r.concept}' is neither canonical nor namespaced`, id);
```

There is **no rule anywhere that a lesson's `concept_tag` must appear in its
spine node's `concepts` array.** Placement is resolved in `curriculum.ts` by:

```ts
const expectedNode = explicitNode !== "" && nodeIds.has(explicitNode)
  ? explicitNode
  : canonicalOwner;
```

The lesson's own `spine_node` frontmatter **wins over the canonical owner**. When
a lesson is placed away from the node that declares its concept, the only cost is
a bookkeeping entry in that node's `relocates` map, which `validateCurriculum`
checks for drift. Spanish's `SPINE-SAY-WHAT-I-DO` ledger already reads:

```json
"relocates": {
  "VERB-SPEAK": "SPINE-POLITE-REQUEST-REPAIR",
  "VERB-WORK":  "SPINE-POLITE-REQUEST-REPAIR",
  "VERB-GO":    "SPINE-ASK-LOCATION"
}
```

So the mechanism for putting a verb at A1 exists, is sanctioned, is validated,
and **is already in use at A1** for `ir`.

For the six candidate verbs named in the original report — `lavar`, `subir`,
`buscar`, `guardar`, `morir`, `enviar` — the obstruction is even less than that.
None exists anywhere in the corpus, and **none has a canonical concept** (the
taxonomy holds 46 `VERB-*` concepts; `VERB-WASH`, `VERB-SEARCH`, `VERB-SEND`,
`VERB-DIE`, `VERB-KEEP`, `VERB-CLIMB` are not among them). They would be
namespaced `ES-VERB-LAVAR` and friends, `conceptOwner` would be `undefined`, and
the `misplaced-shared-realization` branch would never execute. **They face zero
mechanical obstruction of any kind.**

### 3.2 What actually binds is the `canDo` sentence — and it is already breached

The real constraint is editorial: a spine node states a capability in the first
person, and a chapter's `canDo` is composed from it. There is no honest reading
of *"I can understand and produce the cardinal numbers one through five"* that
also teaches *lavar*.

But the honest-filing constraint is **not being honoured today**. The 32 verbal
lessons of §2.4 are hosted as follows:

| node | stage | its `canDo` | what it actually hosts |
|---|---|---|---|
| `SPINE-ASK-LOCATION` | A1 | "I can ask where a familiar person, place, or object is." | the entire plural present paradigm of `-ar`/`-er`/`-ir`, plus `ir` |
| `SPINE-DEFINITE-REFERENCE` | A1 | "I can recognize how this language marks a specific known person or thing…" | gerunds, subject-verb agreement, object pronouns, the infinitive as subject, the imperfect, the preterite |
| `SPINE-POLITE-REQUEST-REPAIR` | pre-A1 | "I can make a request politely and repair a small social mistake." | `hablar`, `trabajar`, `estudiar` and the singular present paradigm |

The original report treated filing verbs under a politeness node as a hypothetical
future compromise — "knowingly filing the entire verb vocabulary of a language
under *make a request politely*". **It is not hypothetical. It is the status quo,
it spans three nodes, two of them at A1, and it covers 32 lessons.** The
invisible lie the report wanted to avoid has already been told.

That reframes the decision. This is not "should we relax a constraint to admit
verbs"; it is "should we keep pretending, or give the material a node that
describes it".

### 3.3 The interlocking debt: `SPINE-SAY-WHAT-I-DO` is 7× over its design target

`SPINE-SAY-WHAT-I-DO` is stage **A2** and declares **42 concepts** — 42 of the
46 canonical `VERB-*` concepts in the taxonomy. `strands.ts` sets
`NODE_CONCEPT_TARGET = 6` with a hard ceiling at `maxNewAtomsPerChapter` (12),
and classifies anything above the ceiling as `over-ceiling`. This node is the
worst offender in the spine by a wide margin.

`HL09` §11 already schedules the repair, as shipping item 5:

> | 5 | Split `SPINE-SAY-WHAT-I-DO` (42 concepts) into rungs of ≤6 | No spine node holds more concepts than a chapter may introduce |

**The verb gap and that unpaid debt are one defect seen from two sides.** Every
canonical verb concept is parked behind a single A2 node that is too big to
realize, which is exactly why verbs cannot reach A1 without a relocation, which
is exactly why the relocations of §3.2 exist.

### 3.4 The constraint that actually costs money

Two rules make any spine change a 23-track change.

`integration.test.ts` requires every track to answer for every node:

```ts
expect(curricula.every((curriculum) =>
  spine.nodes.every((node) => curriculum.spine[node.id] !== undefined),
)).toBe(true);
```

and, for a node a track has not realized, requires the omission to be complete
and exact:

```ts
if (entry!.segments.length === 0) {
  expect([...entry!.omits].sort()).toEqual([...node.concepts].sort());
}
```

`validateCurriculum` enforces the same equality generally via `expectedOmits`.
So **adding one spine node means 23 ledger entries, 22 of which must list every
one of the new node's concepts in `omits`.** That is the unit cost of every
option below, and it is mechanical rather than editorial.

### 3.5 The headroom that constrains the fix: four headwords

Spanish has **304 distinct headwords at or below pre-A1** against a `pre-A1`
target of **300**. Slack: **four**.

This matters because the tidy version of any fix is to move the misfiled verbs
off `SPINE-POLITE-REQUEST-REPAIR` onto a node that describes them. `hablar`,
`trabajar` and `estudiar` are pre-A1 lessons. **Moving three of them up to an A1
node leaves Spanish at 301 and keeps its pre-A1 attainment; moving five breaks
the only level any track has attained.** Any option that relocates pre-A1 verb
lessons upward must budget against those four headwords.

---

## 4. Options

Common to all: 23 track ledger entries per new node (§3.4). Only Spanish is
affected *editorially* — the next-closest track, Telugu, has 222 headwords at or
below A1 and is 378 short of the target, so no other track will realize an A1
node for a long time and all of them simply declare it omitted (§2.6).

A note that applies throughout: because **no track has attained A1 or above**, a
new or altered **A1** node cannot revoke any level any track currently holds. A
new **pre-A1** node would immediately revoke Spanish's pre-A1 until Spanish
realized it. This asymmetry is the single most useful fact for costing.

### Option A — add a new A1 node for everyday actions

A new node, e.g. `SPINE-NAME-EVERYDAY-ACTIONS`, stage A1, strand LEXICON,
`canDo`: *"I can name common everyday actions and say that I do them."* It
declares up to 6 concepts of its own.

- **`core/spine.d/`**: one new file. `SPINE-SAY-WHAT-I-DO` untouched.
- **Tracks affected**: 23 ledger entries; 22 declare full `omits`.
- **Perturbs an attained level?** No.
- **Authoring**: Spanish needs ~16 headwords to reach 600 anyway; one tranche
  (tranches run ~35) can supply the node's realization and close the vocabulary
  criterion together. **~1 tranche.**
- **Against it**: the new node's concepts would have to be *new* canonical
  concepts, because the obvious ones are all owned by `SPINE-SAY-WHAT-I-DO`.
  That leaves 42 verb concepts still parked at A2 and the §3.3 debt untouched,
  and it creates a second place where "which node owns verbs" is answered.
  The existing three relocations stay fictional.

### Option B — widen an existing A1 node's concepts

Add verb concepts to `SPINE-ASK-LOCATION`, which already hosts the verb
morphology in practice, and rewrite its `canDo` to match.

- **`core/spine.d/`**: one file edited; no new node.
- **Tracks affected**: 23 ledgers must recompute `omits` for that node — the
  22 tracks that realize `QUESTION-WHERE` but no verbs must now list the added
  concepts as omitted, so it is not cheaper than Option A in ledger terms.
- **Perturbs an attained level?** No.
- **Authoring**: ~1 tranche.
- **Against it**: it takes the node from 1 concept to 7, over
  `NODE_CONCEPT_TARGET`. Worse, it makes the honesty problem permanent by
  writing it into the spine — the resulting `canDo` has to mean both "ask where
  something is" and "name everyday actions", which is precisely the compound,
  unchosen capability statement this exercise is trying to stop producing. It
  ratifies §3.2 rather than repairing it.

### Option C — take the first rung off `SPINE-SAY-WHAT-I-DO` and land it at A1 *(recommended)*

Execute the first slice of `HL09` §11 item 5. Split ~6 everyday-action concepts
out of `SPINE-SAY-WHAT-I-DO` into a new A1 node, leaving the remainder at A2 for
later slices.

- **`core/spine.d/`**: one new file, one edited (the A2 node's `concepts` list
  shrinks by the moved concepts).
- **Tracks affected**: 23 × 2 entries — a new entry, plus a recomputed `omits`
  for `SPINE-SAY-WHAT-I-DO` (whose omission lists get *shorter*, since the moved
  concepts leave). Roughly double Option A's mechanical cost.
- **Perturbs an attained level?** No — provided at most four pre-A1 verb lessons
  are relocated upward (§3.5). Relocating `hablar`, `trabajar` and `estudiar`
  costs three of the four and is safe; a fourth is the last one available.
- **Authoring**: ~1 tranche, the same one that closes the 16-headword gap.
- **For it**: it is already-scheduled work, so it spends no new design budget.
  It shrinks the spine's worst `over-ceiling` node instead of adding beside it.
  It converts three fictional relocations into honest placements. And it puts
  verbs where the taxonomy already says they belong, rather than minting a
  parallel set of verb concepts at A1.

### Option D — change nothing, and record the composition

Ship the tranche, certify A1 at 600, and add only the gate of §6 so the
composition is *reported* rather than *enforced*.

- **Cost**: near zero.
- **Against it**: it certifies A1 for a five-verb lexicon. It is listed because
  it is the honest baseline against which the others are priced, and because if
  the owner judges §2.5 to mean the harm is tolerable, this is the option that
  follows — but it should be chosen deliberately, which is the whole point.

### Costs at a glance

| | spine.d files | ledger entries | perturbs attainment | tranches | pays down §3.3 |
|---|---|---|---|---|---|
| A — new A1 node | +1 | 23 | no | ~1 | no |
| B — widen `ASK-LOCATION` | 1 edited | 23 | no | ~1 | no |
| **C — first rung off `SAY-WHAT-I-DO`** | **+1, 1 edited** | **46** | **no** | **~1** | **yes** |
| D — do nothing | 0 | 0 | no | 0 | no |

---

## 5. Recommendation

**Option C.**

Option B should be rejected outright: it writes today's misfiling into the spine
as a permanent compound capability, and the spine's `canDo` statements are the
one place in this project where the curriculum's shape is stated in plain
language. Corrupting them to accommodate a filing convenience trades a visible
gap for an invisible one, which is the failure mode the original report correctly
identified.

Option A works and is the cheapest real fix, but it leaves the taxonomy with two
answers to "where do verbs live" and leaves 42 concepts parked behind an
unrealizable A2 node. It buys a year of the same problem.

Option C costs roughly one extra day of mechanical ledger editing over Option A —
23 additional entries, all of them `omits` recomputations that shrink rather than
grow — and in exchange it discharges scheduled debt (`HL09` §11.5), reduces the
spine's worst over-ceiling node, regularizes three standing fictions, and needs
no new canonical concepts because the right ones already exist. The authoring
tranche is the same tranche either way, and it is a tranche Spanish has to write
regardless to reach 600.

The decisions this leaves genuinely open, which are the owner's and not this
document's:

1. **The node's wording.** Something on the order of *"I can name common
   everyday actions and say that I do them."*
2. **Which ≤6 concepts move down.** `VERB-EAT`, `VERB-DRINK`, `VERB-SLEEP`,
   `VERB-LIVE`, `VERB-WALK`, `VERB-BUY` are a defensible everyday-action set;
   an alternative is to move the three already relocated (`VERB-SPEAK`,
   `VERB-WORK`, `VERB-GO`) plus three others, which regularizes the fictions
   immediately but spends three of the four headwords of pre-A1 slack.
3. **Whether the six reported candidates get canonical concepts.** `lavar`,
   `subir`, `buscar`, `guardar`, `morir`, `enviar` currently have none. They can
   ship namespaced against the new node with no taxonomy change at all, or be
   promoted — but promoting them is a cross-language commitment, since a
   canonical concept asks all 23 tracks to eventually answer for it.

---

## 6. The gate that would have caught this

A vocabulary criterion that counts headwords without regard to part of speech
will certify a verbless A1 in every language the project ever adds. The class is
general: **a count with no companion assertion about composition cannot fail for
the right reason.** It is the same shape as a coverage percentage that never asks
*which* lines, or a test count that never asks *which* behaviours.

### 6.1 It is cheap, because the signal already exists

Measured corpus-wide: of 289 `word`/`phrase` lessons whose gloss begins with an
English infinitive ("to …"), **278 — 96.2% — already carry a `VERB` concept
tag**. The eleven that do not are almost all idioms and collocations
(`tomar el pelo`, `pasar a mejor vida`, `cometer un error`) where the headword is
a phrase rather than a verb lexeme, plus four genuine tagging misses (`ver`,
`contar`, `creer`, `explicar`).

So no new frontmatter field, no re-authoring, and no schema change is needed. A
verb is already identifiable from data the validator **already requires every
content lesson to carry**. The residual 4% causes the check to *under*count
verbs, so it errs toward flagging a track rather than certifying one — it fails
safe.

### 6.2 Sketch

In `level-gate.ts`, beside `vocabularyOf`:

```ts
/**
 * Distinct headwords at a level whose concept names a VERB.
 *
 * Criterion 2 counts vocabulary; this counts what KIND. A 600-word A1 made
 * entirely of nouns satisfies the first and fails the learner, so the two
 * numbers have to be asserted separately -- a total can always be reached by
 * the wrong parts.
 */
function verbVocabularyOf(lessons: ParsedLesson[]): number {
  const words = new Set<string>();
  for (const lesson of lessons) {
    if (!CONTENT_TYPES.has(lesson.realization.type)) continue;
    if (!/(^|-)VERB-/.test(lesson.realization.concept)) continue;
    const headword = (lesson.realization.headword ?? "").trim().toLowerCase();
    if (headword) words.add(headword);
  }
  return words.size;
}

/** Verb floor per level. A1 asks the learner to DO things; 40 is ~7% of 600. */
export const LEVEL_VERB_VOCABULARY: Record<CefrLevel, number> = {
  "pre-A1": 5, A1: 40, A2: 120, B1: 250, B2: 400, C1: 800, C2: 1600,
};
```

and, inside the existing per-level loop that already computes `atLevel`, one more
blocker beside the vocabulary one:

```ts
const verbTarget = LEVEL_VERB_VOCABULARY[level];
const verbsAtLevel = verbVocabularyOf(
  trackLessons.filter((l) => atOrBelow(l.realization.lessonId, level)),
);
if (verbsAtLevel < verbTarget) {
  failures.push({
    criterion: "verb-vocabulary",
    detail: `teaches ${verbsAtLevel} distinct verb headwords at or below ${level}, against ${verbTarget}`,
    shortfall: verbTarget - verbsAtLevel,
  });
}
```

Roughly 25 lines plus a constant table and a test. Against today's corpus it
fails Spanish at A1 with `verb-vocabulary(-33)` — seven verb-tagged headwords
against a floor of forty — and would have failed it at every tranche since the
first.

### 6.3 The general form, for HL09 §3.1

The narrow fix is a verb floor. The general fix is a rule for the spec:

> **Every count criterion carries a composition criterion.** Where §3.1 asserts
> *how many*, it must also assert *of what*, for at least one partition of the
> counted set that a reader would care about. A criterion that can be satisfied
> by an arbitrary composition of the counted items is not measuring the thing it
> is named after.

Applied to the existing four criteria: vocabulary counts headwords and should
partition by part of speech (this document). Reinforcement counts revisits and
should partition by *what* is revisited — 83 unrevisited atoms could all be the
same kind. Those are separate pieces of work and are noted here only to show the
rule has more than one customer.

---

## 7. What this document does not do

It changes no spine node, no track ledger, no lesson, and no gate. It adds no
entry to `BACKLOG.d`, `CHANGELOG.d` or `lessons.md`. The measurements in §2 were
taken with throwaway scripts that are not committed; every one of them is
reproducible from the quoted rules and the corpus at `3c442dcf16`.

The next action is the owner's choice among §4, plus the three open decisions in
§5. Implementation follows that choice, per the repo's specs-before-code rule.
