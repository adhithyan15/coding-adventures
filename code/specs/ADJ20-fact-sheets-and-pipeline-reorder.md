# ADJ20 — Fact Sheets and the Engine Pipeline Reorder

## The problem ADJ16 left unsolved

ADJ16 steps 1-5 are on main. The framework can now:

1. Elicit a rulebook from N models (ADJ14, ADJ17).
2. Lower the rulebook into a `LoweredKb` with per-clause
   provenance attribution (step 1).
3. Merge multiple rulebooks via `run_with_rulebooks` (step 2).
4. Detect cross-rulebook disputes via `DisputedAnswer` (step 3).
5. Weight rules by multi-model agreement
   (`compute_agreement_weighted_rulebook`, step 4).
6. Run the engine arm deterministically over the merged KB
   (step 5, `run_engine_arm`).

But there's a load-bearing trick in step 5's demo: the
`tsa_rulebook_strict_ir()` fixture references **`prohibited(matches)`**
directly. The source IR also encodes **`prohibited(matches)`** as
a Fact node. So the engine's "derivation" is `prohibited(matches)`
matches `prohibited(matches)` — a tautology. The framework
produces a verdict, but it didn't actually *reason*.

The reality is that the source declaration just says **"matches"**.
The fact that matches are prohibited under TSA carry-on rules is a
**categorical conclusion** that requires:

- World knowledge: matches are flammable (chemistry).
- Domain rules: flammable items are prohibited (TSA regulation).
- A unification: `flammable(matches) ∧ ∀X: flammable(X) → prohibited(X) ⊢ prohibited(matches)`.

Today's Arm C punts on this. Arms A and B do the leap inside the
LLM's hidden state. **The framework's deterministic-verdict story
is incomplete until the world-knowledge step is itself an
auditable artifact.**

This spec proposes that artifact: **fact sheets**.

## The proposal in one sentence

A **fact sheet** is a typed, audited collection of world-knowledge
facts about a single entity in a single domain, structurally
parallel to a `Rulebook` but holding `Fact` nodes instead of
`Rule` nodes — and reusable across every query that mentions the
entity.

## Fact sheets as first-class entities

```rust
// In adjudication-rulebook or a new adjudication-factsheet crate:

pub struct FactSheet {
    /// Stable identifier (e.g., "matches/tsa-compliance").
    pub entity_id: String,
    /// Domain this fact sheet applies to. The same entity can have
    /// different fact sheets in different domains (e.g., "matches"
    /// in a chemistry pedagogical context vs in TSA compliance —
    /// the relevant facts differ).
    pub domain: String,
    /// IR document carrying Fact nodes only. ADJ02 v3 coverage
    /// constraint applies as usual: every fact spans some source
    /// (here the source is the elicit prompt, not the user's
    /// declaration).
    pub ir_document: serde_json::Value,
    /// Raw text the LLM produced before decomposition. Replay
    /// reproduces ir_document from this + elicit_prompt_version
    /// + model_identity.
    pub source_text: String,
    /// Trust tier; same ladder as Rulebook.
    pub trust: FactSheetTrust,  // Tentative / Reviewed / Authoritative
    /// Where the fact sheet came from.
    pub source: FactSheetSource,
    /// Prompt version, model identity, audit trail — mirror Rulebook.
    pub elicit_prompt_version: String,
    pub decompose_prompt_version: String,
    pub model_identity: ProviderIdentity,
    pub as_of: String,
    pub audit_trail: Vec<LlmCallRecord>,
    pub validation_passed: bool,
    pub validation_error: Option<String>,
}

pub enum FactSheetSource {
    /// Elicited from an LLM via `elicit_subject_facts`. Default
    /// for `acquire_fact_sheet`.
    LlmElicited,
    /// Compiled from an external reference (UpToDate, IATA DGR,
    /// FDA Orange Book, a regulatory document). Highest provenance.
    Reference { citation: String },
    /// Pulled from a structured knowledge graph (Wikidata,
    /// domain ontology). Mid-trust.
    KnowledgeGraph { source_name: String, version: String },
}
```

`FactSheet` is structurally `Rulebook`-shaped but its IR contains
only `NodeKind::Fact` nodes. The trust tier and audit trail follow
the same `Tentative → Reviewed → Authoritative` ladder that ADJ14
defined for rulebooks.

### A worked example: the "matches" fact sheet for TSA compliance

```
entity_id: "matches/tsa-compliance"
domain:    "tsa-compliance"
trust:     Tentative
source:    LlmElicited

source_text:
   1. matches are flammable solids.
   2. matches can ignite from friction.
   3. matches release sulfur dioxide when burned.
   4. matches are commonly categorized as "smoker's materials" in
      international hazmat regulations.
   5. strike-anywhere matches are a sub-category that ignites
      against any rough surface; safety matches require a striker.
   6. matches in their original packaging are not "fireworks" or
      "pyrotechnics" under TSA's hazmat taxonomy.

ir_document:
   F1: flammable_solid(matches)
   F2: ignites_from_friction(matches)
   F3: hazmat_category(matches, "smokers_materials")
   F4: subtype(strike_anywhere_match, matches)
   F5: subtype(safety_match, matches)
   F6: not_in_category(matches, "fireworks")
   F7: not_in_category(matches, "pyrotechnics")
```

The KB the engine sees becomes:

- From source IR: `declared(matches)`, `carry_on(matches)`
- From fact sheet: `flammable_solid(matches)`, etc.
- From rulebook: `prohibited(X) :- flammable_solid(X).`

The engine unifies. The proof DAG is:

```
prohibited(matches)
  ← flammable_solid(matches)   [from fact sheet F1]
  ← (rule head: prohibited(X) :- flammable_solid(X), from rulebook R3)

verdict: NON-COMPLIANT (proof above)
```

Every step is explicit. The categorical leap that used to happen
inside the LLM's forward pass is now an external fact (F1) +
external rule (R3), both auditable.

## The new pipeline order

ADJ16 step 5's order:

1. decompose_text on source.
2. ADJ02-05 checks.
3. Engine runs against `source_ir + rulebook_ir`.

ADJ20's reorder:

1. **decompose_text on source.** Unchanged — produces source IR
   with entity references (e.g., `declared(matches)`).
2. **For each entity in the source IR**:
   - Query the fact-sheet store for `(entity_id, domain)`.
   - On hit: use the cached fact sheet.
   - On miss: elicit a new fact sheet via
     `elicit_subject_facts(entity, domain)`, validate via
     `decompose_text`, store as `Tentative`.
3. **ADJ02-05 checks** on the source IR (unchanged).
4. **Acquire rulebook** (`acquire_rulebook` or
   `acquire_rulebook_adversarial`). Unchanged.
5. **Engine run**: lower source IR + fact sheets + rulebook into a
   combined KB via `run_with_rulebooks`; run queries.
6. **Verdict + proof DAG**, with provenance attributing each cited
   clause to its source rulebook OR fact sheet OR the source
   declaration itself.

The reorder puts entity-fact enrichment **between source extraction
and engine query** — exactly where the categorical leap needs to
happen.

## The new primitive: `elicit_subject_facts`

```rust
// In llm-primitives:

pub struct ElicitSubjectFactsRequest {
    pub entity: String,        // "matches"
    pub domain: String,        // "tsa-compliance"
    pub language_hint: Option<String>,
}

pub struct ElicitSubjectFactsResponse {
    pub raw_text: String,      // 6-12 numbered facts
    pub call_record: LlmCallRecord,
}

pub fn elicit_subject_facts<G: Gateway>(
    req: &ElicitSubjectFactsRequest,
    gateway: &G,
) -> Result<ElicitSubjectFactsResponse, PrimitiveError>;
```

The prompt template:

```
You are a domain expert in {domain}. The framework will apply
formal rules to a query that mentions "{entity}". Your task is
to list the world-knowledge facts about "{entity}" that the
rules will need to fire correctly.

Rules of fact listing:
1. Each fact must be true regardless of the specific query —
   facts ABOUT {entity}, not facts about a particular use.
2. Each fact must be relevant to {domain} adjudication.
   Don't list facts that no plausible rule in {domain} would
   reference (e.g., "matches were invented in 1827" is true but
   irrelevant to TSA compliance).
3. Each fact must be defensible by citation to authoritative
   reference material in {domain}.
4. Number each fact starting from 1.
5. Aim for 6-12 facts. More than 12 risks the fact-sheet
   reviewer overlooking marginal facts; fewer than 6 risks
   missing categorical reasoning the engine will need.

Example for entity="ibuprofen", domain="emergency-medicine":
1. ibuprofen is an NSAID.
2. ibuprofen inhibits COX-1 and COX-2 pathways.
3. ibuprofen is contraindicated in active GI bleeding.
4. ibuprofen has no specific antidote; supportive care for
   overdose per AAPC poison control.
5. ibuprofen onset of action is 30-60 minutes orally.
...

Now produce the fact sheet for entity="{entity}",
domain="{domain}":
```

The prompt explicitly anchors facts to (a) reference-grade
sources and (b) the domain's adjudication needs. This is the
same prompt discipline that `elicit_rules` uses for rule
elicitation; the difference is the OBJECT (entity vs rule corpus)
and the SCOPE (specific entity's properties vs general domain
rules).

## `acquire_fact_sheet`: the orchestrator

```rust
// In adjudication-rulebook or a new adjudication-factsheet crate:

pub struct AcquireFactSheetRequest {
    pub entity: String,
    pub domain: String,
    pub as_of: String,
    pub language_hint: Option<String>,
}

pub fn acquire_fact_sheet<G: Gateway>(
    req: &AcquireFactSheetRequest,
    gateway: &G,
) -> Result<FactSheet, AcquireFactSheetError>;
```

Mirrors `acquire_rulebook`:

1. Call `elicit_subject_facts` to get raw text.
2. Call `decompose_text` to type the IR (Fact nodes only).
3. Validate via `adjudication_ir::validate`.
4. Wrap as `FactSheet { trust: Tentative, source: LlmElicited, ... }`.

### The adversarial variant (parallels ADJ17)

```rust
pub fn acquire_fact_sheet_adversarial<G: Gateway>(
    req: &AcquireFactSheetRequest,
    gateways: Vec<(String, G)>,  // (model_label, gateway) pairs
) -> Result<AdversarialFactSheet, AcquireFactSheetError>;
```

Same shape as `acquire_rulebook_adversarial`: each model elicits
independently, fact sheets get merged via
`compute_agreement_weighted_factsheet` (analogue of
`compute_agreement_weighted_rulebook`), the result is a single
fact sheet where each `Fact(...)` lowering carries a weight
equal to `count_of_models_with_this_fact / total_models`.

Cross-model agreement on a fact ("matches are flammable") is a
**much** stronger signal than agreement on a rule, because facts
are entity-bounded and verifiable against external references.
If gemma4 + llama3.1 + qwen2.5 all say `flammable(matches)`,
that's three different model families agreeing — the disagreement
budget for that fact is essentially zero.

## The fact-sheet store

```rust
pub trait FactSheetStore {
    fn lookup(&self, entity: &str, domain: &str) -> Option<&FactSheet>;
    fn insert(&mut self, sheet: FactSheet);
    fn promote(
        &mut self,
        entity: &str,
        domain: &str,
        new_trust: FactSheetTrust,
        reviewer: &str,
    ) -> Result<(), PromoteError>;
}
```

Two reference implementations:

- `InMemoryFactSheetStore` — `HashMap<(String, String), FactSheet>`,
  ephemeral. Useful for tests and single-process demos.
- `DiskFactSheetStore` — backed by a directory with one JSON file
  per `(entity, domain)` pair. The default for the demo binary so
  fact sheets accumulate across runs.

The store is **not yet a network service** — single-process,
filesystem-backed is sufficient for the framework's current
deployment story. A future cluster-shared store could be a
network service exposing the same trait.

### Lookup-or-elicit semantics

```rust
fn ensure_fact_sheet<G: Gateway, S: FactSheetStore>(
    entity: &str,
    domain: &str,
    store: &mut S,
    gateway: &G,
) -> Result<&FactSheet, AcquireFactSheetError> {
    if let Some(existing) = store.lookup(entity, domain) {
        return Ok(existing);
    }
    let req = AcquireFactSheetRequest { ... };
    let sheet = acquire_fact_sheet(&req, gateway)?;
    store.insert(sheet);
    Ok(store.lookup(entity, domain).unwrap())
}
```

The first query for `(matches, tsa-compliance)` elicits and
stores. Every subsequent query reuses. This is the framework's
analogue of memoization — the LLM call is paid once per
(entity, domain) per `as_of` window, not per query.

For a TSA deployment with N queries per day mentioning common
items (matches, water bottles, lithium batteries), the elicit cost
amortizes to near zero after the first few weeks. The model is in
the loop *at fact-sheet authoring time*, not at answer time.

## Integration with what's on main

The plumbing on main from ADJ16 steps 1-5 already accepts
`&[(IRDocument, ClauseProvenance)]` as rulebook inputs to
`run_with_rulebooks`. Fact sheets are structurally
`(IRDocument, ClauseProvenance)` pairs too — just with
`NodeKind::Fact` content instead of `NodeKind::Rule` content.

**No changes to `run_with_rulebooks` are needed.** The new
orchestrator's job is to:

1. Source IR → typed and validated.
2. For each entity referenced by source IR, call
   `ensure_fact_sheet`.
3. Build the rulebooks slice: `[(source_ir, source_provenance),
   (fact_sheet_1.ir, fact_sheet_1_provenance), ...,
   (rulebook.ir, rulebook_provenance)]`.
4. Call `run_with_rulebooks` with the assembled slice.

The existing `ClauseProvenance.trust_tier` field handles the
mixed-trust case: source IR is `Authoritative` (the declaration is
authoritative for itself), fact sheets are `Tentative` (until
reviewed), rulebook is whatever its trust tier is. The engine
arm's `clause_provenance` table already distinguishes these.

## Provenance attribution flows through

The proof DAG today already cites `FactId` and `RuleId` per
proof. After ADJ20, attribution becomes:

- `FactId(0)` → `ClauseProvenance { source_rulebook_id: "doc1",
    trust_tier: Authoritative }` (the source's `declared(matches)`)
- `FactId(1)` → `ClauseProvenance { source_rulebook_id:
    "matches/tsa-compliance", trust_tier: Tentative }` (the fact
    sheet's `flammable_solid(matches)`)
- `RuleId(0)` → `ClauseProvenance { source_rulebook_id:
    "tsa-strict-v1", trust_tier: Reviewed }` (the rulebook's
    `prohibited(X) :- flammable_solid(X).`)

The verdict's audit trail tells a reviewer exactly:
**"the engine concluded NON-COMPLIANT because the source declared
matches; the matches fact sheet (tentative) says matches are
flammable solids; the TSA rulebook (reviewed) says flammable solids
are prohibited."** Three separately reviewable artifacts; one
deterministic proof.

## Implementation sequence

Roughly mirrors ADJ16's six-step pattern.

1. **ADJ20-impl-1**: Define `FactSheet`, `FactSheetTrust`,
   `FactSheetSource` types. Add to a new `adjudication-factsheet`
   crate or extend `adjudication-rulebook`.
2. **ADJ20-impl-2**: Implement `elicit_subject_facts` primitive
   in `llm-primitives`. Just the prompt + JSON response + error
   types.
3. **ADJ20-impl-3**: Implement `acquire_fact_sheet` orchestrator.
   Mirrors `acquire_rulebook`'s structure.
4. **ADJ20-impl-4**: Implement `FactSheetStore` trait +
   `InMemoryFactSheetStore` + `DiskFactSheetStore`.
5. **ADJ20-impl-5**: New pipeline entry point
   `run_with_fact_sheets` (or extend `run_with_rulebooks`) that
   takes the source IR, looks up / elicits fact sheets for
   referenced entities, and dispatches to the engine.
6. **ADJ20-impl-6**: Add `acquire_fact_sheet_adversarial` +
   `compute_agreement_weighted_factsheet`.
7. **ADJ20-impl-7**: TSA demo Arm D — full pipeline with fact
   sheets + rulebook + engine. The first verdict the framework
   can produce that's actually defensible end-to-end.

Each impl PR is small (mirrors a proven shape from ADJ14 or
ADJ16). The conceptual lift is mostly in this spec.

## Open questions

1. **Where do entity references come from?** Step 2 of the new
   pipeline says "for each entity in the source IR" — but the IR
   grammar (ADJ01 v3) has a `NodeKind::Entity` that's reserved
   for deduplicated reference targets. Today's `decompose_text`
   doesn't emit `Entity` nodes routinely; the source IR has Facts
   that mention atoms (e.g., `prohibited(matches)`). Do we:
   - (a) Extract entity references from Fact node terms
     (post-hoc analysis).
   - (b) Update the decompose_text prompt to require explicit
     Entity nodes for every entity in the document.
   - (c) Have a separate primitive
     `extract_entities_from_ir(source_ir) -> Vec<String>` that
     walks the IR and returns the set of entities to look up
     fact sheets for.

   (c) is cheapest; (b) is most rigorous but requires a
   prompt-version bump and re-validation of every existing IR.
   Defer to impl-1's design discussion.

2. **What's the domain key for a fact sheet?** The example uses
   `"matches/tsa-compliance"` but the relationship between
   `entity` and `domain` is arbitrary today. Should we:
   - (a) One canonical fact sheet per (entity, domain) — the
     proposal above.
   - (b) Multiple fact sheets per entity, indexed by
     (entity, scope) where scope is finer-grained than domain
     (e.g., `matches/tsa-carry-on` vs
     `matches/tsa-checked-baggage`).
   - (c) A single canonical fact sheet per entity with
     domain-specific RULES doing the relevant scoping.

   (c) is closest to how Wikipedia works (one article per
   entity, multi-section). (a) is closest to how hazmat manuals
   work (one entry per item per regulatory regime). (b) is
   neither. Defer.

3. **Stratification with `Tentative` fact sheets.** If the
   engine's proof DAG cites a `Tentative` fact, the verdict's
   trust upper-bound is `Tentative`. The framework should refuse
   to mark a verdict as `Authoritative` if any cited clause is
   `Tentative`. This is a small extension to
   `ResolutionRequirement` (ADJ16 step 3) — needs a new variant
   like `RequiresFactSheetPromotion { entity_id, domain }`. Defer
   to impl-5.

4. **Fact sheet evolution over time.** TSA's rules are
   relatively stable; chemistry doesn't change at all. But
   medical knowledge moves: a 2020 fact sheet for "remdesivir"
   would say "investigational antiviral"; a 2024 one would say
   "FDA-approved for COVID-19 treatment". The `as_of` field
   captures the temporal window. The store needs versioning so
   replaying a 2022 adjudication uses the 2022 fact sheet, not
   the 2024 one. Same pattern as ADJ09's rule `as_of` semantics;
   this spec inherits that solution.

5. **Cross-domain fact-sheet reuse.** Some facts are
   domain-independent (`flammable(matches)` is true in TSA AND
   in chemistry AND in customs). Should the store deduplicate
   across domains? Probably yes, but the current proposal's
   (entity, domain) key forces re-elicitation. A `World-knowledge`
   sub-store with un-domain-scoped facts could be a future
   addition; defer.

## Why this isn't just "RAG"

A retrieval-augmented-generation pipeline retrieves text
fragments and stuffs them into the LLM's context, then asks the
LLM to reason over them. ADJ20 is structurally different:

- **Retrieval is to the engine, not the LLM.** The fact sheet
  is lowered into Prolog facts that the engine unifies against.
  The LLM only sees the fact sheet during decomposition (which is
  itself audited).
- **Facts are typed and validated.** They're not free-text
  snippets; they're IR Fact nodes that have been through ADJ02
  coverage validation. Every fact has a span back to the
  authoring prompt.
- **Provenance is enforced.** Every cited fact carries
  `(source_fact_sheet_id, trust_tier)`. A reviewer can promote
  individual facts (or whole fact sheets) and the proof DAG
  changes accordingly.
- **Replay is exact.** Same source + same fact sheets + same
  rulebook = byte-for-byte identical verdict. RAG-based reasoning
  has the same non-determinism the LLM-answer path has (token
  jitter, ordering effects).

RAG is good engineering for prototype QA systems. ADJ20 is what
RAG would be if you wanted to defend the verdict in court.

## See also

- [ADJ14](ADJ14-rule-elicitation.md) — the rule-elicitation
  primitive this spec parallels.
- [ADJ16](ADJ16-engine-programmatic-adjudication.md) — the
  engine pipeline ADJ20 enriches.
- [ADJ17](ADJ17-adversarial-rulebook-empirical-results.md) —
  the adversarial-elicitation pattern that
  `acquire_fact_sheet_adversarial` reuses.
- [ADJ18](ADJ18-broadened-tsa-empirical-bench.md),
  [ADJ19](ADJ19-cross-domain-empirical-bench.md) — once ADJ20
  ships, the bench shape extends with a fourth Arm A mode that
  injects fact sheets into the prompt directly (analogue of
  `fixture-priming` but with fact-sheet context), AND with an
  Arm C bench that runs the engine over the full source +
  fact-sheet + rulebook KB.
