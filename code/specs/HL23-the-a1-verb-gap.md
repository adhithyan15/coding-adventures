# HL23 — The A1 verb gap

**Status:** decision document, **amended twice after implementation** — see §8 and §9. The
owner chose Option C (§4). The first slice is implemented; §8 records the
constraint that made this document's costing of Option C wrong, and re-prices
the remainder. Sections 1-7 are left as written, because a decision document
that is quietly edited to agree with what happened is not evidence of anything.

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

---

## 8. Amendment — what Option C actually costs *(added after the first slice)*

§4 priced every option at "23 ledger entries per new node", and §3.4 named the
two rules that make a spine change a 23-track change. **There is a third rule,
it is the expensive one, and this document missed it.**

### 8.1 The rule

`curriculum.ts`, in the pass that classifies every lesson sitting in a path
segment:

```ts
const isSharedContent =
  CONTENT_TYPES.has(lesson.realization.type) &&
  conceptOwner.get(lesson.realization.concept) === placement.node;
const count = extensionLessonCount.get(lessonId) ?? 0;
if (!isSharedContent && count === 0) {
  error(
    "unclassified-curriculum-extension-lesson",
    `${curriculum.language}: ${lessonId} is local support but belongs to no extension node`,
  );
}
```

A lesson in a path segment is either the **canonical realization** of one of
that node's own concepts, or it is **local support** and must belong to an
extension node. There is no third category.

So moving a concept from node X to node Y **reclassifies every lesson realizing
it**. A lesson that was shared content of X becomes local support of X, and
unless it is already inside an extension, it is an error. §3.1 was therefore
right that nothing forbids a verb at A1 and right that `relocates` is sanctioned
— but `relocates` only works for a lesson that is *already packaged as an
extension*, which is why `ES-C10-ir` survives and `ES-C07-comer` does not.

The consequence for Option C: **a concept cannot leave `SPINE-SAY-WHAT-I-DO`
unless every lesson realizing it, in every track, moves with it or is demoted
into an extension.** That is not a ledger edit. It is a lesson migration.

### 8.2 The measured price, per concept

Realizing lessons that are **not** already extension-resident — i.e. the lessons
that would have to be moved or re-packaged before the concept can be released:

| concept | lessons to move | tracks | note |
|---|---|---|---|
| `VERB-CAN`, `VERB-GIVE`, `VERB-PUT` | **0** | — | free; all realizers already extension-resident |
| `VERB-INFINITIVE`, `VERB-PRESENT-HABITUAL` | **0** | — | free, but grammatical — they *are* the A2 node's `canDo` |
| `VERB-DRINK` | 1 | spanish | the only everyday action costing a single track |
| `VERB-BUY`, `VERB-PLAY`, `VERB-WAIT`, `VERB-MEET`, `VERB-ANSWER`, `VERB-BRING`, `VERB-GET` | 2 | italian, spanish | |
| `VERB-SLEEP`, `VERB-WALK`, `VERB-OPEN`, `VERB-CLOSE`, `VERB-RUN`, `VERB-SIT`, `VERB-STAND` | 3 | french, german, spanish | |
| `VERB-SEE` | 4 | gujarati, malayalam, punjabi, sanskrit | |
| `VERB-EAT`, `VERB-COME`, `VERB-KNOW` | 5 | +gujarati, malayalam, punjabi, sanskrit | |
| `VERB-LIVE` | 5 | french, german, hindi, italian, spanish | **plus two hard errors** — see below |
| `VERB-THINK`, `VERB-UNDERSTAND`, `VERB-READ`, `VERB-WRITE`, `VERB-TAKE`, `VERB-ASK`, `VERB-HELP`, `VERB-LIKE-LOVE` | 9 | nine tracks each | |

`VERB-LIVE` is worse than its row suggests. `FR-C05-habiter` and `GE-C05-wohnen`
carry no explicit `spine_node`, so re-parenting the concept also changes their
`canonicalOwner` and they fail `misplaced-shared-realization` as well —
a second, independent error class on the same two lessons.

**§4's estimate of Option C was wrong by a category, not by a margin.** The
recommended six-concept set (`VERB-EAT`, `VERB-DRINK`, `VERB-SLEEP`,
`VERB-LIVE`, `VERB-WALK`, `VERB-BUY`) costs **19 lesson migrations across 13
tracks**, plus two `misplaced-shared-realization` repairs. A PR series is not a
cheaper option that got more expensive; it is a different option, and it should
be chosen deliberately rather than inherited from a number in a table.

### 8.3 What the first slice actually shipped

`SPINE-NAME-EVERYDAY-ACTIONS`, stage A1, strand LEXICON:

> **I can name common everyday actions.**

carrying **three** concepts — `VERB-DRINK`, `VERB-GIVE`, `VERB-PUT`. Chosen
because they are the everyday actions releasable without editing another track:
the first costs one Spanish-only segment split (`ES-PATH-029` held exactly two
lessons, so `ES-C07-beber` became `ES-PATH-A1-BEBER` under the new node), and
the other two cost nothing at all.

`SPINE-SAY-WHAT-I-DO` goes **42 → 39**. That is a real slice of `HL09` §11 item
5 and visibly not the whole of it; 39 is still more than three times the chapter
atom ceiling, so the node stays the spine's worst offender and stays pinned.

Deliberately **not** done, and the reasons matter:

- **The 26 misfiled A1 grammar lessons stay where they are.** The plural present
  paradigm under `SPINE-ASK-LOCATION`, and the gerunds, imperfect, preterite,
  subject-verb agreement and infinitive-as-subject under
  `SPINE-DEFINITE-REFERENCE`, are **morphology**, and this node's `canDo` claims
  only **naming**. Relocating them would have required widening the `canDo` to
  cover both — which is precisely the compound, unchosen capability statement
  §5 rejected Option B for producing. They are misfiled today, this node does
  not fix them, and the fix is a GRAMMAR rung of its own in a later slice.
- **The three standing fictions stay too.** `VERB-SPEAK` and `VERB-WORK` are
  pre-A1 lessons, and §3.5's four headwords of slack is the entire budget any
  future slice has; `VERB-GO` was left with them so the trio moves together
  when a rung exists that honestly covers it.
- **No canonical concept was minted.** The six candidate verbs of §5 decision 3
  ship namespaced. Promoting them asks all 23 tracks to answer for them, which
  is a commitment this slice has no reason to make on their behalf.

---

## 9. Amendment — the second slice, and what §8.2 does not price *(added after the DELE sitting)*

§8 was written before anything had been *sat*. `mocks/a1/sitting-2026-08-26.md`
then sat both A1 mocks and returned **`NO APTO` on both**, with Grupo 1 at 3,00
and 0,00 against a 30,00 bar. **62 of its 86 failed objective items involve a
missing high-frequency verb.** That converts this document's subject from a
composition concern into a measured failure, and it changes which concepts are
worth what.

### 9.1 §8.2's table has holes, and they are where the value is

§8.2 prices 30 of the node's concepts. It is silent on six: `VERB-BE`,
`VERB-HAVE`, `VERB-DO-MAKE`, `VERB-SAY`, `VERB-HEAR` and `VERB-LEARN`. Three of
those are on the sitting's most-missed list. Measured the same way — realizing
lessons that are not already extension-resident:

| concept | Spanish | lessons to move | tracks | note |
|---|---|---|---|---|
| `VERB-LEARN` | *(none)* | 1 | german | `GE-C05-lernen` carries no explicit `spine_node` |
| `VERB-SAY` | `decir` | 1 | persian | Spanish realizer is already extension-resident |
| `VERB-DO-MAKE` | `hacer` | 2 | german, hindi | `GE-C05-machen` carries no explicit `spine_node` |
| `VERB-HAVE` | `tener` | 3 | french, german, spanish | **empties `GE-PATH-018`**; `FR-C14-avoir` and `GE-C14-haben` carry no explicit `spine_node`, so both take a second `misplaced-shared-realization` error — the `VERB-LIVE` shape again |
| `VERB-HEAR` | `oír` | 3 | french, german, spanish | |
| `VERB-BE` | `ser` | 8 | french, german, gujarati, malayalam, persian, punjabi, sanskrit, spanish | **empties `GE-PATH-021`** |

The consequential line is `VERB-DO-MAKE`. It ties for the **most-needed verb on
the exam** (9 items, with `gustar`) and costs **two** foreign lessons — cheaper
than every 3-track row §8.2 does price. A table that omits it understates the
best move available.

### 9.2 What the second slice shipped

Four concepts moved to `SPINE-NAME-EVERYDAY-ACTIONS`, stage A1, strand LEXICON,
whose `canDo` — *"I can name common everyday actions"* — already covers all four
with **no wording changed**:

`VERB-DO-MAKE` (`hacer`, 9 items) · `VERB-BUY` (`comprar`, 5) ·
`VERB-OPEN` (`abrir`, 4) · `VERB-CLOSE` (`cerrar`, 3).

Price paid: **11 lesson migrations across 5 tracks** (french, german, hindi,
italian, spanish), plus 23 tracks × 2 nodes of realization ledger. Spanish goes
**617 → 621** headwords and **40 → 44** verbs at or below A1. **pre-A1 stays at
304** — §3.5's four headwords of slack are untouched, because this slice moves
A2 → A1 only.

`SPINE-SAY-WHAT-I-DO` goes **39 → 35** concepts. Still the spine's worst
`over-ceiling` node, still pinned.

### 9.3 Deliberately not done, and why

- **`poder` and `tener` stay at A2**, though they are the 4th and 3rd
  most-missed verbs and `VERB-CAN` costs literally nothing in foreign lessons.
  Being *able* to do a thing is not an everyday action, and *having* a thing is
  not one either; putting them on this node means widening its `canDo` to cover
  naming **and** ability **and** possession, which is precisely the compound,
  unchosen capability statement §5 rejected Option B for producing. They need
  their own rungs. The syllabus already asks for one: PCIC `A1-F2-16` (*ask
  about ability*) and `A1-F2-17` (*express ability*) are enumerated A1 points
  and both are currently `probe: null` — **unmapped** — in
  `exam-inventory-es-a1.json`. A `SPINE-SAY-WHAT-I-CAN-DO` rung is therefore
  justified by the inventory rather than invented to hold a verb.
- **`porque` stays at B1.** It sits under `SPINE-NARRATE-EVENTS`, not under this
  node, so it is a different migration with a different price.
- **`gustar` stays at A2.** Joint most-needed verb on the exam, and the single
  most expensive row in §8.2: `VERB-LIKE-LOVE` is 9 lessons across 9 tracks.
  It deserves a slice of its own rather than a corner of this one.

### 9.4 The re-sat result, and the thing it settles

Both mocks were re-sat against the new corpus with the same harness and the same
rule. **Both still return `NO APTO`.**

| | Grupo 1 | Grupo 2 | |
|---|---|---|---|
| mock 1, before → after | 4,00 → **5,00** / 50 | 11,58 → **11,58** / 50 | NO APTO |
| mock 2, before → after | 0,00 → **5,17** / 50 | 5,00 → **11,67** / 50 | NO APTO |

Objective items failed: 84 → **82** of 100.

This is the expected shape, not a disappointment: the sitting's own §5 already
granted **all 34** high-frequency verbs as a counterfactual and still failed both
groups in both mocks. Its measured minimum is ~136 lexemes — about 34 verbs
**and about 100 everyday nouns**. So §4's framing needs one correction carried
forward: **re-staging the verb cluster is necessary and is not sufficient.**
A future slice that moves the remaining verbs and stops there will still sit
`NO APTO`, and should say so in advance rather than discover it.

One methodological note, because it bears on how much weight the numbers above
carry. The sitting's scoring scripts were scratchpad artifacts (§8 of that
document) and were not committed. They were rebuilt from its description and
calibrated against its published result before use: the reconstruction
reproduces the pinned 689 headwords / 756 lessons at or below A1, 810 headwords
and 78 verbs at or below A2, and the project's own 40-verb count, and it
reproduces **six of the eight published paper scores exactly** — every
production paper, and both of mock 2's group totals. It differs on two by
exactly one objective item each, in the generous direction. The before/after
figures above are therefore both taken on the reconstruction, so the comparison
is like-for-like even where the reconstruction and the original disagree.

---

## 10. Amendment — the ability rung *(added after the third slice)*

§9.3 left `tener` and `poder` at A2 on purpose, and named the condition for
moving them: they need **a rung of their own**, because
`SPINE-NAME-EVERYDAY-ACTIONS` cannot absorb them without its `canDo` becoming a
compound capability statement. This slice builds that rung.

### 10.1 The node

`SPINE-SAY-WHAT-I-HAVE-AND-CAN-DO`, stage **A1**, strand **FUNCTION**:

> **I can say what I have and what I am able to do.**

carrying **`VERB-HAVE`** (`tener`, the sitting's single most-missed verb — 6
objective items and 2 production tareas) and **`VERB-CAN`** (`poder`, 3 items
and 2 tareas).

Two things about that sentence, because it is the part this document exists to
police.

It is **two exponents, not two capabilities.** The test HL23 applies is not "one
clause" — `SPINE-EXCHANGE-NAMES` is three clauses and `SPINE-CHECK-WELLBEING`
two, and each names one transaction. The test is whether the statement is a
*grab bag*: whether a later concept could be filed under it for convenience.
"Naming actions **and** possession **and** ability" is unbounded, which is why
§9.3 refused it. This node's sentence is bounded to two closed classes and
licenses nothing else.

And the strand is **FUNCTION rather than LEXICON**, unlike
`SPINE-NAME-EVERYDAY-ACTIONS`, because the node exists to close two enumerated
PCIC *function* points and not to give two verbs somewhere to live.
`SPINE-SAY-WHAT-I-WANT` is the precedent: a FUNCTION node carrying one lexical
concept, `VERB-WANT`.

**One divergence from §9.3, stated rather than buried.** §9.3 says `poder` and
`tener` "need their own rung**s**", plural. This slice builds **one**. The
reason is the paragraph above — the two are bounded together and the sentence
stays honest — and the cost of the alternative is real: a second node is a
second 23-track realization ledger for one concept. If a later slice finds the
sentence being stretched, the split is cheap and the ledger machinery is now
written; that is the trigger to watch for, not a number.

### 10.2 What it cost, and where §9.1 over-priced it

§9.1 priced `VERB-HAVE` at **3 lessons across french, german and spanish**, plus
two `misplaced-shared-realization` repairs, and noted that it **empties
`GE-PATH-018`**. Measured against the corpus, the first half holds and the
second does not, for a reason worth recording:

**a concept can move without its lesson moving, if the lesson's whole SEGMENT
moves instead.** Level is derived from the segment's `spine_node`, and a
segment's position in path order is independent of the stage its node declares —
which is why `ES-PATH-030-HACER-CH12` already sits at path index 61 while being
an A1 segment. Retargeting a segment therefore costs one line and preserves
`curriculum-prerequisite-order` exactly, because nothing moves.

| track | lesson | what actually happened |
|---|---|---|
| spanish | `ES-C08-tener` | `ES-PATH-030-TENER` **split**: `ES-PATH-A1-TENER` takes the word lesson to the new node; `ES-PATH-030-TENER-PLURAL` keeps `tenemos`/`tienen` at A2 |
| spanish | `ES-C11-poder` | `ES-PATH-030-ABILITY-CH11` **retargeted** — the segment held only this lesson |
| french | `FR-C14-avoir` | `FR-PATH-016` **split**: `FR-PATH-A1-AVOIR` takes `avoir`; `FR-C14-age` stays at A2 with its extension |
| german | `GE-C14-haben` | `GE-PATH-018` **retargeted** — one line, and it is **not emptied** |

So `GE-PATH-018` survives, and the two `misplaced-shared-realization` repairs
§9.1 predicted never arise: `FR-C14-avoir` and `GE-C14-haben` carry no explicit
`spine_node`, so their `canonicalOwner` follows the concept to the new node — and
the new node is now where they sit. The italian, latin and portuguese realizers
are extension-resident and were free, exactly as §9.1 said.

**The plural paradigm deliberately stays at A2.** `ES-C08-tenemos` and
`ES-C08-tienen` are morphology, and this node's `canDo` claims saying what you
have, not conjugating it — the same line §8.3 drew for the 26 misfiled A1 grammar
lessons.

### 10.3 The two inventory points, and why closing them meant authoring a verb

§9.3 justified this rung on `A1-F2-16` (*ask about ability*) and `A1-F2-17`
(*express ability*) being enumerated A1 points that were **unmapped**. Building
the node does not by itself close them, and the inventory says why in its own
notes:

> *The source's A1 exponent is `saber` plus a noun or infinitive. The corpus
> teaches `no sé` only as a fixed repair phrase and never introduces `saber` as a
> verb.* … *The corpus teaches `poder` for capability but never `saber` as a
> verb, and substituting it would be a different structure.*

Those notes were written by someone who had already considered pointing the two
points at `poder` and refused. Overriding that to make a coverage number move is
the exact failure mode this document exists to prevent, so the points were closed
the only honest way: **chapter 389 authors `saber`**, introducing `ES-LEX-SABER`
and `ES-GRAMMAR-SABER-INFINITIVO`, and both points now probe those two atoms.
They share a probe, which is the inventory's own convention — `A1-F2-13` and
`A1-F2-14` already share `ES-LEX-CONOCER-10` for exactly the ask/express pair.

Coverage moves **223 → 225 of 273**, unmapped **50 → 48**. The percentage does
**not** move: two points in 273 is 0.7pp and rounds away. That is worth naming,
because a headline percentage standing still is not evidence that nothing
happened — which is why `covered` and `unmapped` are pinned beside it.

`saber` is **not** on any mock item's `requires:` line. It buys nothing on the
exam and was authored anyway, because the rung's stated justification was these
two points, and a justification that is not discharged is a justification that
was decoration.

### 10.4 The counts

| | before | after |
|---|---|---|
| headwords ≤ **pre-A1** | 304 | **304** |
| headwords ≤ **A1** | 621 | **624** |
| verbs ≤ **A1** | 44 | **47** |

pre-A1 is untouched: §3.5's four headwords of slack are all still there. The
three new A1 headwords are `tener`, `poder` and `saber`.

`SPINE-SAY-WHAT-I-DO` goes **35 → 33** concepts. Still the spine's only
`over-ceiling` node, still pinned, and the pin still only ever falls.

### 10.5 The re-sat result — `NO APTO` again, and Grupo 1 does not move at all

| | Grupo 1 | Grupo 2 | |
|---|---|---|---|
| mock 1, before → after | 4,00 → **4,00** / 50 | 11,58 → **12,58** / 50 | NO APTO |
| mock 2, before → after | 5,17 → **5,17** / 50 | 11,67 → **13,33** / 50 | NO APTO |

Objective items failed: 83 → **82** of 100.

**Grupo 1 does not move by a single point on either mock**, and that is the
finding rather than a disappointment. Releasing the two most-missed verbs in the
corpus bought exactly one objective item, and it was in *auditiva*. Every reading
item that wanted `tener` or `poder` wanted a noun as well:

- mock 1 #2 wanted `tener` — and `terraza` and `habitación`;
- mock 1 #13 wanted `tener` — and `aeropuerto` and `necesitar`;
- mock 1 #23 wanted `poder` — and `ordenador` and `internet`.

§9.4 said re-staging verbs is necessary and not sufficient. This slice measures
*how* insufficient: the verb famine and the noun famine are **multiplicative on
the same items**, so verb work alone will keep returning a Grupo 1 in single
figures no matter how many verbs it releases. The remaining verb backlog
(`gustar`, `querer`, `preferir`, `porque`) should therefore be sequenced *with*
the noun tranches, not before them.

### 10.6 The harness, and one residual it did not talk itself out of

Same instrument as §9.4, rebuilt from `sitting-2026-08-26.md` §8's description
and recalibrated before use. It reproduces **689 / 756 / 810 / 40** on the
pre-#13132 corpus, matches the committed `core/level-snapshots/spanish.json`
histogram exactly, and reproduces the sitting's published ten most-missed lexemes
(`gustar` 9, `hacer` 9, `tener` 8, `comprar` 5, `poder` 5, …) to the number. The
band rule is not invented either: `sitting-2026-08-26.md` §5.3 states it — *band
0 when more than 40 % of required lexis is missing* — and applying it with the
`!` points inside the denominator reproduces all eight production cells in both
states, and §3.1's writing-band claim as well.

**It differs from §9.4's reconstruction on one cell: mock 1 *lectura*, where it
reads 4 and §9.4 read 5.** Exactly five mock-1 reading items are blocked by a
single lexeme apiece — `universidad`, `médico`, `barato`, `llevar`, `actividad` —
and granting any one of them reproduces §9.4's figure and every other calibration
target at once. None of the five exists anywhere in the Spanish corpus: not as a
headword, not as an atom id, and not as body text in any of the 756 lessons at or
below A1. So the residual was left standing rather than closed by granting one,
because granting one would have been inventing the evidence. This instrument is
therefore **one objective item stricter** than §9.4's on that cell, which is the
safe direction, and the before/after columns above are both taken on it.

An A2-verb count residual is recorded for the same reason: the harness reads 77
where the sitting published 78, and the only way to reach 78 is to widen the type
filter until `ES-C66-la-terminacion-dice-quien` counts as a verb — it matches
`(^|-)VERB-` solely because its concept is `ES-SUBJECT-VERB-AGREEMENT`. That
lesson sits at A1, so admitting it would give 41 verbs at or below A1 and break
the project's own pinned 40. No rule yields both. The shipped code path was kept.

---

## 11. Amendment — the staging slice, and two published price tables that are wrong *(added after the fourth slice)*

§10.5 measured the thing that decides this document's remaining plan: **the verb
famine and the noun famine are multiplicative on the same items.** Before
authoring anything, all four bundles were scored:

| granted | mock 1 G1 | mock 2 G1 | result |
|---|---|---|---|
| nothing (baseline) | 4,00 | 5,17 | NO APTO |
| the ~100 missing nouns **alone** | **28,33** | **26,33** | **NO APTO** |
| the 17 staged-above-A1 entries **alone** | 13,33 | 7,17 | NO APTO |
| **both** | **33,33** | **33,33** | **APTO** |

Authoring every noun the exam asks for and stopping there lands Grupo 1 **1,67
and 3,67 short of the 30,00 bar**. That is the whole authoring budget spent for a
measured `NO APTO`. The staging half is not an optimisation; it is load-bearing,
and this slice ships it first so every authoring tranche after it is measured on
top.

### 11.1 Three rungs, and one node that was simply mis-staged

| node | stage | strand | `canDo` | carries |
|---|---|---|---|---|
| `SPINE-NAME-EVERYDAY-THINGS` *(new)* | A1 | LEXICON | I can name common everyday things. | `LODGING-ROOM` |
| `SPINE-SAY-WHAT-I-LIKE` *(new)* | A1 | FUNCTION | I can say what I like and what I do not like. | `VERB-LIKE-LOVE` |
| `SPINE-SAY-WHY` *(new)* | A1 | FUNCTION | I can give one reason for something. | `CONNECTIVE-BECAUSE` |
| `SPINE-NAME-EVERYDAY-ACTIONS` | A1 | LEXICON | **unchanged** | +8 action verbs |

`gustar` got a rung of its own rather than being absorbed by the everyday-action
node — liking is not an action — and `porque` got one rather than dragging B1's
`SPINE-GIVE-REASONS` ("*reasons and explanations for my opinions and plans*")
down to A1. Eight verbs — ask, stand, write, wait, read, bring, come, walk —
moved onto `SPINE-NAME-EVERYDAY-ACTIONS` **with no `canDo` change at all**,
because every one of them is a common everyday action.

**The finding worth more than the rungs: `SPINE-SAY-WHAT-I-WANT` was mis-staged.**
Its `canDo` is *"I can say what I want or need, and ask for it"* — an A1
capability by any reading, and one DELE A1 tests directly. It sat at **A2**, and
it sat there because it declared `SPINE-SAY-WHAT-I-DO` (A2) as a prerequisite.
But *quiero un café* needs one memorised form, not the present-tense machinery
that node teaches. So the node is restaged **A2 → A1**, the spurious prerequisite
is dropped, and **its `canDo` is not touched, because nothing was wrong with it.**

That is HL23's own thesis recurring in a node nobody had looked at: a capability
the learner needs early, parked high by a dependency that was assumed rather than
checked. **This document has audited concepts and never audited node stages, and
there is no reason to think this is the only one.** A stage audit against the
declared prerequisites is cheap and is now recorded in `BACKLOG.d`.

### 11.2 §8.2 and §9.1 are wrong, and here are the corrected numbers

Both tables priced a concept move as *"realizing lessons that are not already
extension-resident"*. §10.2 showed that over-counts, because **a concept moves
without its lesson moving when the whole segment is retargeted** — level derives
from `segment.spine_node`, and a segment's path position is independent of the
stage its node declares. Measured across the corpus, the real price has three
outcomes per realizer, not one:

- **RETARGET** — the segment holds nothing that stays. One line, no lesson moves,
  `curriculum-prerequisite-order` cannot break because nothing is reordered.
- **FREE** — the realizer is already extension-resident.
- **SPLIT** — the segment is cut into runs of consecutive lessons sharing a
  destination.

| concept | §8.2/§9.1 said | realizers | free | retarget | split |
|---|---|---|---|---|---|
| `VERB-ASK` | 9 lessons, 9 tracks | 20 | 11 | 0 | 9 |
| `VERB-WRITE` | 9 lessons, 9 tracks | 20 | 11 | 0 | 9 |
| `VERB-READ` | 9 lessons, 9 tracks | 20 | 11 | 0 | 9 |
| `VERB-LIKE-LOVE` | 9 lessons, 9 tracks | 20 | 11 | 0 | 9 |
| `VERB-COME` | 5 lessons, 5 tracks | 15 | 10 | 0 | 5 |
| `VERB-WALK` | 3 lessons, 3 tracks | 4 | 1 | 0 | 3 |
| `VERB-STAND` | 3 lessons, 3 tracks | 4 | 1 | 0 | 3 |
| `VERB-WAIT` | 2 lessons, 2 tracks | 4 | 2 | 0 | 2 |
| `VERB-BRING` | 2 lessons, 2 tracks | 4 | 2 | 1 | 1 |
| `VERB-HAVE` (§9.1) | 3 lessons **+ 2 repairs + empties `GE-PATH-018`** | 6 | 3 | 1 | 2 |

**Two corrections matter beyond the arithmetic.**

First, §9.1's `VERB-HAVE` row predicted two `misplaced-shared-realization`
repairs and an emptied `GE-PATH-018`. **Neither happened.** `GE-PATH-018` held
only `GE-C14-haben`, so it was retargeted in one line and still exists; and
because `FR-C14-avoir` and `GE-C14-haben` carry no explicit `spine_node`, their
`canonicalOwner` followed the concept to the new node, which is where they now
sit. The predicted error class never arose.

Second — and this is what changes the plan — **the four 9-track concepts are the
same twenty realizers sitting in the same segments.** `ES-PATH-031` alone holds
`preguntar`, `leer`, `escribir` and `gustar`. Moving them **together** costs one
split per segment instead of one per concept, so the marginal price of the fourth
concept is nearly zero. §8.2 lists `VERB-LIKE-LOVE` as its single most expensive
row and §9.3 deferred `gustar` on that basis; measured properly, `gustar` is
almost free **provided it travels with its neighbours**. A slice that moves these
one at a time pays four times over for nothing.

### 11.3 One invariant the run-splitter broke, and the completion that fixes it

Splitting a segment into runs preserves lesson order exactly, which is what
`curriculum-prerequisite-order` needs. It is not sufficient. `validateCurriculum`
holds two more invariants: an extension is attached to **exactly one** segment,
and every lesson it names lives in that segment. Cutting a segment across an
extension's lesson set breaks both, and it did — **173 errors**, of the shapes
`AR-EXT-027-LANGUAGE-SPECIFIC uses AR-C28-jaa outside AR-PATH-027` and
`… is attached to both AR-PATH-027 and AR-PATH-027-B`.

The fix is not special-casing: it is applying the same operation to the
extension. An extension whose lessons land in *n* runs becomes *n* extensions,
each attached to the run holding its lessons. **A segment split is an extension
split.** That is now the rule, and it is the second time this document has
recorded that a split is more expensive than it looks — the first was Italian's
`IT-PATH-024` needing three ways rather than two.

### 11.4 The counts, and one debt that grew

| | before | after |
|---|---|---|
| headwords ≤ **pre-A1** | 304 | **304** |
| headwords ≤ **A1** | 624 | **638** |
| verbs ≤ **A1** | 47 | **58** |

`SPINE-SAY-WHAT-I-DO` goes **33 → 24** concepts.

**`SPINE-NAME-EVERYDAY-ACTIONS` joins the `over-ceiling` list at 15 concepts, and
that is stated rather than absorbed.** Nine concepts left a 33-concept node for a
7-concept one and the destination crossed the 12 ceiling on the way, so
over-ceiling concepts went **33 → 39**. The debt redistributed *and* grew. The
alternative was leaving eight everyday-action verbs at A2 where the exam that
asks for them cannot reach them, and the real fix is HL-C81's split of both
nodes, not a different filing. The pin now names both nodes.

`cansado`, `estudiante` and `lado` were deliberately **left behind**. Each would
have needed an adjectives-and-people rung with no honest canonical concept to put
on it, and minting one to hold three namespaced lessons is the widening this
document exists to refuse. Ablation confirms APTO still holds without them.

### 11.5 The re-sat result

| | Grupo 1 | Grupo 2 | |
|---|---|---|---|
| mock 1, before → after | 4,00 → **12,33** / 50 | 12,58 → **18,33** / 50 | NO APTO |
| mock 2, before → after | 5,17 → **7,17** / 50 | 13,33 → **13,33** / 50 | NO APTO |

Objective items failed: 82 → **78** of 100.

`NO APTO`, exactly as §11's own bundle table predicted for staging alone — 12,33
and 7,17 against a predicted 13,33 and 7,17, the small difference being the three
entries deliberately left behind. The written paper moves for the first time in
the series, 0,00 → 8,33 on mock 1, because `gustar`, `porque` and `querer`
between them lift both of its tareas out of band 0.

Grupo 1 is still less than half the bar. The ~103 authored lexemes are what close
it, and the bundle table says they close it exactly.

---

## 12. Amendment — two decisions the bundle proof forced *(added at the first authoring tranche)*

§11.5 ended "The ~103 authored lexemes are what close it, and the bundle table says they
close it exactly." The bundle proof run at the head of the first authoring tranche
**reproduces that result and then finds two conditions on it that §11 does not state.**
Both were escalated and both were decided; this section records the decisions and, more
importantly, why neither is a reversal of an earlier judgement.

### 12.1 The measurement

Harness as §10.6, byte-identical, mocks and keys untouched. Baseline reproduces §11.5
exactly — mock 1 Grupo 1 **12,33**, mock 2 **7,17**, 78 objective items failed.

| granted | mock 1 G1 | mock 2 G1 | |
|---|---|---|---|
| all 103 | **31,33** | **32,33** | **APTO** |
| 103 + the three §11.4 left behind | 32,33 | **33,33** | APTO |
| 103 − the ten that fail the confusability screen | 30,33 | **28,33** | **NO APTO** |
| 103 − the pure adjectives | 30,33 | **26,33** | **NO APTO** |

§11's 33,33 / 33,33 reconciles exactly: the gap is `cansado`, `estudiante` and `lado`, and
restoring them returns mock 2 to 33,33 and mock 1 to one objective item short — the
documented §10.6 residual on mock 1 *lectura*. **The plan is arithmetically sound and was
never in doubt. What was missing is that it assumed all 103 are authorable, and ten are
not on this document's own rules.**

### 12.2 Decision one — build an A1 rung for qualities

`barato`, `gratis`, `mayor`, `importante`, `favorito`, `caro`, `menor` are adjectives. The
A1 spine carries `SPINE-NAME-EVERYDAY-THINGS` and `SPINE-NAME-EVERYDAY-ACTIONS` and nothing
that hosts a quality.

The measured cost of that hole: of the fourteen mock-2 reading items still failing after a
complete noun-and-verb tranche, **seven are blocked by an adjective and nothing else** —
`caro` (items 2 and 3), `barato` (8), `importante` (10), `mayor` (15 and 22), `gratis` (6),
`adulto` (11). A greedy search over the *entire* clean noun/verb pool plateaus mock 2's
Grupo 1 at **20,33** against a 30,00 bar. **No quantity of noun authoring passes mock 2.**

**This is not a reversal of §11.4, and the distinction matters.** §11.4 declined to mint an
adjectives rung, and was right to: it was offered a rung with no honest canonical content,
holding three namespaced lessons, minted to give three words somewhere to live. That is the
widening this document exists to refuse, and the refusal stands on its own facts.

**The premise has since changed in two independent ways.** First, the content is now
exam-derived rather than convenient — every candidate is on the list because a measured item
requires it. Second, and decisively, the syllabus itself asks for the rung:
`exam-inventory-es-a1.json` enumerates **`A1-NG6-03`** (*attractiveness: guapo, feo,
bonito*), **`A1-NG6-08`** (*interest*) and **`A1-NG6-10`** (*ease and difficulty*) as A1
points, and all three are `probe: null` — **unmapped**. That is the same test §10.1 applied
to the ability rung, which was justified by `A1-F2-16`/`A1-F2-17` rather than by needing
somewhere to put `poder`. A rung the source enumerates is justified by the source.

Its `canDo` must be a single honest capability about describing what something is like, and
it must not become a grab bag — the test §10.1 states is not "one clause" but whether a
later concept could be filed under it for convenience.

**What is explicitly refused:** tranche 6's option of filing adjectives under
`SPINE-COUNT-ONE-TO-FIVE` (*"the cardinal numbers one through five"*). It is available, it
is cheap, and it is precisely the mis-filing this programme exists to undo. Recorded here so
that the next tranche does not rediscover it as a shortcut.

### 12.3 Decision two — the confusability screen disambiguates; it does not drop

Tranche 6 derived the rule — *a same-length pair differing in one position is a drop only
when the differing position is not the first* — and could afford it, because it screened
roughly a hundred candidates to place thirty-five.

Ten of the 103 flag: `costar`/`contar`, `caro`/`cero`+`cara`, `tren`/`tres`, `gorro`/`gordo`,
`mayo`/`mano`+`malo`, `menor`/`menos`, `playa`/`plaza`, `pollo`/`polvo`, `recto`/`resto`,
`amigo`/`amiga`. Dropping all ten costs the exam.

**The rule did not change; the pool did.** On an exam-derived list every entry is
load-bearing, so a drop forfeits an item rather than selecting between interchangeable
candidates. Refusing to teach `tren` because `tres` exists is not a defensible A1
curriculum, and it teaches around a difficulty the learner will meet anyway.

So the screen's verdict is **context-dependent, and that is now part of the rule**:

- **drop** when the candidate pool has surplus;
- **disambiguate** when the candidate is required.

`tren`/`tres`, `pollo`/`polvo`, `playa`/`plaza`, `costar`/`contar` are minimal pairs, and
presenting the pair and making the contrast the lesson is the standard way to teach
discrimination. That converts all ten liabilities into assets.

**The general form, and it is the reusable half:** a screen calibrated against an abundant
candidate pool encodes an unstated assumption that substitutes exist. **A screen that always
drops silently shrinks the curriculum toward whatever is easy to teach**, and never reports
that it has made a curricular decision. Re-derive a filter's *action* — not its criterion —
whenever the pool stops being abundant.

### 12.4 A fifth duplicate miss-mode, and two indexes rather than one

The four already-owned traps on record — `llevar`, `andar`, `dar`, `llover` — are each
invisible to a headword-only screen. **`amigo` is a fifth kind: it is owned by
`ES-C09-falsos-amigos`, whose headword is the two-word term of art *falsos amigos*, at A2.**
No headword screen, atom-id screen or root ledger finds it; only a screen that decomposes
multiword headwords into component words does.

It is also the case where "already owned" is arguably the wrong answer, since the corpus
teaches the metalinguistic term *false friends* and not the word *friend* — so whether it is
a duplicate or a genuine gap is a judgement no ledger records.

And the decomposition that catches it must **not** be reused for confusability. The two
questions need two indexes: *is this already taught?* takes the **wide** index, where a
fragment counts as ownership; *will a learner conflate this?* takes the **narrow** index of
whole displayed headword forms, because a fragment was never presented as a word. Conflating
them produced a false drop on `menor` against `mejor`, which occurs only inside the idiom
*pasar a mejor vida*.
---

## 13. Amendment — the qualities rung, and a refusal that was written forward-looking *(added at the second authoring tranche)*

§12.2 decided that an A1 rung for qualities would be built, justified by three unmapped
inventory points rather than by convenience. This section records what building it cost,
what it closed, and the thing it found on the way — which is worth more than the rung.

### 13.1 The bundle proof, re-run, and the fourth consecutive exact prediction

Harness as §10.6, byte-identical to the copies both prior slices used, mocks and keys
untouched. Baseline reproduces §12's published post-tranche-7 state exactly — mock 1
Grupo 1 **26,33**, mock 2 **20,33**, 52 objective items failed — and the corpus census
reproduces **304 / 673 / 68** two independent ways.

| granted | m1 G1 | m1 G2 | m2 G1 | m2 G2 | | failed |
|---|---|---|---|---|---|---|
| baseline | 26,33 | 22,25 | 20,33 | 15,33 | NO APTO | 52 |
| the seven qualities alone | 27,33 | 24,25 | 24,33 | 15,33 | NO APTO | 45 |
| **all 68 authorable** | **31,33** | **34,25** | **32,33** | **34,00** | **APTO** | **6** |
| all 68 − the seven qualities | 30,33 | 32,25 | **25,33** | 33,00 | NO APTO | 17 |
| all 68 − the nine confusability flags | 30,33 | 31,25 | **28,33** | 29,00 | NO APTO | 19 |

**Both §12 decisions are independently load-bearing**, and the proof now says so with the
qualities actually authored rather than merely granted. Dropping them returns mock 2 to
25,33; dropping the nine flagged candidates returns it to 28,33; the bar is 30,00.

The strongest evidence that the pool is right is arithmetic rather than argument: tranche 7
authored **35** and this document's remaining pool is **68**, and granting all 68 on top of
those 35 returns **31,33 / 32,33** — the row §12.1 published for "all 103", to the cent.

### 13.2 The node

| | |
|---|---|
| id | `SPINE-DESCRIBE-QUALITIES` |
| stage | A1 |
| strand | LEXICON |
| `canDo` | **I can say what something is like.** |
| prerequisites | `SPINE-NAME-EVERYDAY-THINGS` |
| concepts | **empty** |

**It is the first node in the corpus with no canonical concept, and that is the point.**
A node's `concepts` list is where it makes a claim on all 23 tracks; minting `QUALITY-CHEAP`
would ask twenty-two tracks to answer for a concept nobody asked them for. This rung is
justified by the *Spanish* A1 syllabus, so it asks nothing of anyone else, and its lessons
carry namespaced `ES-QUALITY-*` tags exactly as tranche 7's did. `validateCurriculum` permits
this; no node had simply needed it before.

The §10.1 test — not "is the `canDo` one clause" but "could a later concept be filed here for
convenience" — is met: a quality adjective belongs, and a noun, a verb or a speech act does not.

**What is explicitly not done:** §12.2's refused shortcut of filing adjectives under
`SPINE-COUNT-ONE-TO-FIVE`. See §13.4, which is about discovering that it had already happened.

### 13.3 The price nobody had priced: 575 renames

Shard filenames are **positional** — element *i* is named `(i+1)*10`. Inserting an A1 node into
the middle of the ladder therefore renumbers every shard after it, in `core/spine.d/` and in all
23 tracks' `curriculum.d/spine/`. That is **575 renames**, plus one hand-edit for `marwadi`,
which is unsharded by design.

§11.1 minted three nodes and paid this silently; it is recorded here because the alternative is
tempting and wrong. Appending the node after the C2 nodes costs nothing structurally and
`check:shards` would pass — but the shard module's own contract says the spine's keys "follow the
pre-A1 → C2 ladder in all 23 tracks", so appending buys a permanent lie about the curriculum's
shape to avoid a one-off mechanical cost. Git renders the whole thing as pure renames.

One trap for whoever does this next: `marwadi/curriculum.json` is a monolith, so a new node has
to be inserted **at its ladder position**, not appended to the object. Appending it passes
`check:shards` — which never reads marwadi — and fails `curriculum-shards` with a key-order
diff forty lines long.

### 13.4 The finding: a refusal written forward-looking, and twenty-two instances already behind it

§12.2 closed with a warning:

> **What is explicitly refused:** tranche 6's option of filing adjectives under
> `SPINE-COUNT-ONE-TO-FIVE` … Recorded here so that the next tranche does not rediscover it
> as a shortcut.

**It had already happened, at scale.** When that paragraph was written, the numbers node — *"I
can understand and produce the cardinal numbers one through five"* — was carrying **twenty-two
quality adjectives**: `alto`, `largo`, `ancho`, `corto`, `lleno`, `pequeño`, `gordo`, `joven`,
`lento`, `dulce`, `seco`, `mojado`, `fresco`, `tibio`, `grueso`, `simpático`, `alegre`, `feo`,
`último`, `necesario`, `entero`, `escaso`. Chapters 365, 371, 379 and 380 are adjectives and
nothing else.

The refusal was phrased about the future, so nobody looked backwards. **A rule adopted after a
defect exists protects only the future unless the same change sweeps the past** — and the rule's
own existence is evidence the defect was attractive enough to commit at least once.

So this slice relocates chapters **365, 371, 379 and 380** onto the new rung. All four are wholly
adjectival, so each is §11.2's cheapest class — a one-line segment retarget, no lesson moves, no
splits, no extension splits — and the move is score-neutral because both nodes are A1. It makes
**both** nodes' `canDo` true: the numbers node stops claiming `alegre` and `necesario`.

Chapters 344, 351 and 352 are **mixed** and are left alone; `entero` and `escaso` sit inside
`ES-PATH-352-01` beside three quantity nouns, so moving them needs a segment split and therefore
— per §11.3 — an extension split. That is a different job and is in `BACKLOG.d`.
`ES-PATH-359-01` (`metro`, `gramo`, `kilo`, `litro`, `peso`) was examined and left on purpose.

**This is the second instance of the shape in this programme.** §11.1 found
`SPINE-SAY-WHAT-I-WANT` mis-staged at A2 by an unchecked prerequisite — right contents, wrong
place; this is right place, wrong contents. Both were found while doing something else. A
one-off audit asking *which nodes hold content their `canDo` does not describe* is in `BACKLOG.d`.

### 13.5 Four inventory points, one of which cost no authoring at all

| point | closed by |
|---|---|
| `A1-NG6-03` attractiveness | `guapo`, `bonito` — and `feo`, which the corpus **already had** |
| `A1-NG6-08` interest | `interesante` |
| `A1-NG6-10` ease and difficulty | `fácil`, `difícil` |
| `A1-NG6-09` capacity with *saber* | **nothing — its note had gone stale** |

Unmapped points **48 → 44**; A1 coverage **225 → 229 of 273**, 82% → **84%**.

`A1-NG6-09` is the one to read twice. Its note said *"the corpus never introduces saber as a
verb, only the fixed phrase no se"* — which stopped being true the moment §10.3 authored `saber`
for `A1-F2-16`/`A1-F2-17`. The atom `ES-LEX-SABER` had existed since #13154 and nothing pointed
at it. **A justification that silently expires is a live hazard, not a tidy-up**, because the
`note` field is exactly what the inventory test trusts as proof that a null was considered.

`feo` deserves its own line: it was to have been authored here, and it is the one word of the
thirteen that was **already taught** — under `ES-COUNT-UGLY`, on the numbers rung, which is how
§13.4 was found at all.

### 13.6 The screen missed `feo`, and arithmetic caught it

The screen ran over the **76 exam-derived candidates** and reproduced §12.3's ten confusability
pairs and all five §12.4 duplicate miss-modes exactly, including `amigo` inside *falsos amigos*.
It then missed a duplicate, for two reasons worth separating:

- **It was never pointed at the word.** Six exponents were added by this slice to close the
  inventory points, and the screen was run over the exam-derived pool only. **Anything a tranche
  will author is a candidate**, including words added to satisfy a coverage target.
- **Re-running it after authoring cannot help**, because the screen then reports the tranche to
  itself: every new word comes back `already owned`, owned by the tranche's own files. A duplicate
  screen must read the corpus **as it stood before the tranche**.

`feo` was caught because the headword count came out **685** where twelve new lessons predicted
686. A corpus-wide duplicate-headword invariant — one line, case-folded, no set to be pointed at
— is now in the pre-commit sweep, and it is worth more than the bespoke screen it backstops.

One new confusability flag also appeared that no earlier screen could have seen:
**`llevar`/`llegar`**, because tranche 7 authored `llegar`. **Authoring manufactures
confusability**, so the pair check has to re-run against the corpus as it now stands rather than
against the candidate list as it was drawn.

### 13.7 The counts

| | before | after |
|---|---|---|
| headwords ≤ **pre-A1** | 304 | **304** |
| headwords ≤ **A1** | 673 | **685** |
| verbs ≤ **A1** | 68 | **68** |
| lessons on the qualities rung | — | **32** (20 relocated, 12 new) |
| unmapped inventory points | 48 | **44** |

The pre-A1 floor is untouched, which it must be; no verb was authored, so the verb count does not
move; and the twelve new headwords are twelve, not thirteen, because of `feo`.

### 13.8 The re-sat result

| | Grupo 1 | Grupo 2 | |
|---|---|---|---|
| mock 1, before → after | 26,33 → **27,33** | 22,25 → **24,25** | NO APTO |
| mock 2, before → after | 20,33 → **24,33** | 15,33 → 15,33 | NO APTO |

Objective items failed: 52 → **45**.

`NO APTO`, and exactly the figures §13.1's own table projected for the qualities alone — 27,33
and 24,33, to the cent, the fourth consecutive slice where the bundle proof predicted the outcome
before the prose was written. Mock 2's Grupo 1 moves **+4,00**, the largest single-slice movement
on that paper in the series, which is what §12.2 meant by *"no quantity of noun authoring passes
mock 2."*

Tranches B and C — the remaining **61** lexemes — are what close the rest.

That figure is 68 authorable minus the **seven** exam-derived qualities this tranche shipped, not
minus the twelve lessons it wrote: `interesante`, `fácil`, `difícil`, `bonito` and `guapo` were
never in the 68, because no mock item requires them. They are there to discharge §13.5's inventory
points. Measured rather than inferred, the harness now reports **69** still-missing lexemes, of
which **8** are the already-owned entries of §12.4 — 61 authorable, and the two roads agree.
