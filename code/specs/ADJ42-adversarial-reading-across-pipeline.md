# ADJ42 — Adversarial Reading Across the Pipeline

> ADJ05 specified adversarial verification narrowly: take the IR
> rendering of a leaf node (from the ADJ04 round-trip check), have
> a *different* model find a contradicting reading, have a judge
> rule on plausibility. That mechanism is correct and load-bearing
> — but it was scoped to one stage of the pipeline.
>
> **The same adversarial-reading mechanism should apply at every
> point where the framework commits to an interpretation.** Not
> just round-trip renderings: rulebook claims, citation
> interpretations, verdicts, kickback decisions. Each of these is
> a place where a single model's "reasonable interpretation" might
> be one of several equally-plausible readings the framework
> shouldn't silently default to.
>
> ADJ42 generalizes ADJ05 across the pipeline. Specifies which
> commit points get adversarial passes, what the adversarial-pass
> output looks like at each point, how disagreements escalate to
> kickback, and how the audit trail records every adversarial
> exchange.

## The reframe

ADJ05 said: "at one stage of the pipeline, check the model's
rendering against an adversary." That's the right shape for the
narrow problem (did the renderer drift from source?) but
incomplete as a discipline.

The framework's broader thesis — the *attention scaffold* — says:
LLMs hallucinate when they're allowed to glide over features.
Adversarial reading at a commit point forces the system to attend
to the *other valid interpretations* it might be glossing over.

There are at least **six commit points** in the pipeline where a
single model's interpretation is currently treated as authoritative
but where alternative valid readings exist. Each is a hallucination
risk. Each deserves an adversarial pass.

## The six commit points

### 1. IR extraction (input)

**What's committed**: the Extractor model assigns a `kind` (Fact /
Uncertainty / Query / Discarded) to each phrase. Decides what's a
domain claim vs. discardable filler.

**What could be wrong**: a phrase classified as `Discarded:Pleasantry`
might actually be a `Fact` (or vice versa). A `Fact` could have
been better classified as `Uncertainty`.

**Adversarial reading**: a different model receives the same input
phrase + the original model's classification. Question: "is there
a different valid kind for this phrase, with a different downstream
implication?"

**Resolution**: if the adversary finds a plausible different kind,
the framework either (a) re-runs extraction with a synthesized
disambiguation prompt or (b) kicks back to the human if the
disagreement persists.

### 2. Round-trip rendering (ADJ04, the original ADJ05 surface)

**What's committed**: the renderer's natural-language paraphrase
of an IR node back to text.

**What could be wrong**: the rendering may smuggle in connotations
not in the source.

**Adversarial reading** (existing ADJ05): a different model produces
a *contradicting* rendering of the same IR node; the judge ranks
which is more plausible against the source bytes.

**Resolution**: existing ADJ05 mechanism — drift detected via
NLI plus adversary's reading + judge ruling.

### 3. Rulebook claims (LR values)

**What's committed**: when the rulebook is LLM-elicited (per
ADJ37), each `contributes(LR, evidence, conclusion)` clause is
the LLM's stated LR for an evidence-to-conclusion link.

**What could be wrong**: the LR value might be reasonable per one
interpretation of the cited literature but wrong per another.
(Example from ADJ37: my elicited rulebook had patient-factor LRs
calibrated for "any delirium" but the input's query was
"med-induced delirium specifically" — the LR magnitudes were
right for one interpretation, wrong for the other.)

**Adversarial reading**: a different model receives the same
rulebook prompt and produces its own LR + citation. The two LRs
are compared; if they diverge by more than a threshold (e.g.,
log-odds difference > 0.5), the framework flags the rule as
adversarially-uncertain.

**Resolution**: either (a) accept the more conservative LR with
provenance from both models, (b) treat the rule as Uncertainty
(don't apply it), or (c) kick back to a human with both readings.
Configurable per deployment.

### 4. Citation claim-match (ADJ40)

**What's committed**: the framework's NLI-style judgment that the
cited source supports the citing claim ("the case actually says
what you say it says").

**What could be wrong**: the NLI primitive could rate a Weak match
as Partial or vice versa. The single LLM's interpretation of "the
case says X" could be one of several valid readings.

**Adversarial reading**: a different model performs the same
claim-match. Independent strength assessments. If they agree
→ confidence boosted; if they disagree → the citation is treated
as having an unresolved interpretation, kicked back.

**Resolution**: this is essentially ADJ40's `ClaimMatchClient`
extended with cross-model agreement requirements.

### 5. Verdict commitment

**What's committed**: the engine's final posterior probability +
inferred verdict, packaged with the proof DAG and the recommended
action tier (e.g., "P(ACS) = 49% → urgent workup tier").

**What could be wrong**: the verdict's *interpretation* — what
action tier the posterior implies — could differ between models.
A model might say "49% → urgent workup" while another says "49%
→ observation with serial troponins; not yet urgent."

**Adversarial reading**: a different model receives the same proof
DAG (without the original model's recommended action tier) and
produces its own tier recommendation. If they disagree → kick
back to the clinician with both interpretations.

**Resolution**: human decides which interpretation is right for
the case at hand. This is where genuine human judgment is
appropriate; the framework facilitates rather than overriding.

### 6. Kickback question selection

**What's committed**: the framework's choice of *which* unresolved
atom to ask the human about, when multiple atoms have similar VOI.

**What could be wrong**: the VOI computation might tie or
near-tie. The selected question might not be the most actionable.
Or another atom might be more important *clinically* even if its
mathematical VOI is slightly lower.

**Adversarial reading**: a different model evaluates the same
unresolved-atom set and produces its own ranking. If the top
question differs → present both questions to the human, let them
choose.

**Resolution**: the human picks; the framework records which
question they engaged with for future calibration.

## The unified interface

```rust
pub trait AdversarialReader: Send + Sync {
    /// Name of this adversary (e.g., "claude-3.5-sonnet",
    /// "gpt-4o", "llama-3.1-70b-instruct"). Used for audit trail.
    fn name(&self) -> &str;

    /// What kind of commit point this adversary handles.
    fn commit_point(&self) -> CommitPoint;

    /// Generate an alternative reading at the given commit point.
    async fn read_adversarially(
        &self,
        context: AdversarialContext,
    ) -> AdversarialReading;
}

pub enum CommitPoint {
    IrExtraction { phrase: String, original_kind: ClaimKind },
    RoundTripRendering { ir_node_id: NodeId, original_rendering: String },
    RulebookClaim { rule_id: RuleId, original_lr: f64 },
    CitationClaimMatch { citation_id: NodeId, original_strength: MatchStrength },
    VerdictTier { proof_dag: ProofDAG, original_tier: String },
    KickbackQuestion { unresolved_atoms: Vec<NodeId>, original_top: NodeId },
}

pub struct AdversarialContext {
    pub original_interpretation: serde_json::Value,  // domain-specific
    pub source_bytes: Option<Vec<u8>>,                // for round-trip cases
    pub model_independence_required: bool,            // (vendor, family) match
}

pub struct AdversarialReading {
    pub alternative_interpretation: serde_json::Value,
    pub justification: String,                        // why this is alternative-valid
    pub confidence: AdversarialConfidence,
    pub agrees_with_original: bool,                   // primary boolean output
    pub disagreement_kind: Option<DisagreementKind>,
}

pub enum DisagreementKind {
    /// The adversary's reading is incompatible with the original's
    Incompatible,
    /// Both readings are plausible but lead to different downstream
    /// inferences
    Divergent,
    /// The original is one of multiple equally-valid readings the
    /// framework should have flagged as ambiguous
    AmbiguousSourcedOverConfident,
}

pub enum AdversarialConfidence {
    High,    // the adversary is confident in its alternative
    Medium,
    Low,
}
```

## The judge layer

Where ADJ05 has a separate `Plausibility` judge model, ADJ42
generalizes: each commit point has its own judge protocol because
the question being judged is different.

| Commit point | Judge question |
|---|---|
| IR extraction | "Which classification is more defensible: original or adversary's?" |
| Round-trip rendering | "Which rendering is more faithful to the source bytes?" |
| Rulebook claim | "Which LR is better supported by the cited literature?" |
| Citation claim-match | "Does the source unambiguously support the claim, or are both readings valid?" |
| Verdict tier | "Both posteriors round to the same value; do the recommended action tiers differ in a clinically/legally meaningful way?" |
| Kickback question | "Which question is more actionable given the case context?" |

The judge model **MUST** be independent of both the original and
the adversary. Default deployment: original = primary extractor,
adversary = secondary model from different family, judge = third
model from third family. (Cost: 3 LLM calls per commit point; only
worth it for high-stakes adjudications. Configurable.)

For lower-stakes cases, the framework can configure a 2-model
mode: original + adversary, where disagreement automatically
kicks back without a judge. Cheaper, more conservative.

## Independence requirement

ADJ05's `(vendor, model_family)` independence requirement extends
verbatim to ADJ42:

- Original model and adversary MUST be from different `(vendor,
  model_family)` tuples
- Judge (when present) MUST be from a third `(vendor, model_family)`
- The framework's `GatewayConfig::check_independence` enforces this
  before any adversarial pass runs

This prevents the model from "agreeing with itself" via shallow
copies, fine-tunes, or model-family relatives.

## Cost model

Adversarial reading triples (or doubles, in 2-model mode) the LLM
call count for any commit point it's enabled at. This is real cost.
The framework's deployment policy controls which commit points get
adversarial passes:

| Commit point | Default adversarial-pass | When to enable |
|---|---|---|
| IR extraction | ON | Always — cheap classification, high impact |
| Round-trip rendering | ON | Existing ADJ04/05 |
| Rulebook claim | OFF (rulebook elicitation is offline) | When rulebook is fresh from LLM |
| Citation claim-match | ON for primary claims; OFF for "see also" | When `claimed_proposition` is set |
| Verdict tier | ON for high-stakes domains | Medical, legal, financial |
| Kickback question | OFF (cheap to defer to human) | When multiple VOI ties |

This is configurable per deployment via a policy object.

## The audit trail records every adversarial exchange

Every adversarial pass produces an `AdversarialRecord` in the
audit trail:

```rust
pub struct AdversarialRecord {
    pub commit_point: CommitPoint,
    pub original_model: String,
    pub adversary_model: String,
    pub judge_model: Option<String>,
    pub original_reading: serde_json::Value,
    pub adversary_reading: serde_json::Value,
    pub judge_ruling: Option<serde_json::Value>,
    pub outcome: AdversarialOutcome,
    pub timestamp: String,
    pub cost_units: u32,
}

pub enum AdversarialOutcome {
    Agreement,                                   // both readings agree
    Disagreement { resolved_by: ResolvedBy },    // disagreement → kickback or judge
    JudgeRuledForOriginal,
    JudgeRuledForAdversary,
    JudgeDeclinedToRule { reason: String },
}

pub enum ResolvedBy {
    JudgeModel,
    HumanKickback,
    DeploymentPolicy { rule: String },           // "always conservative"
}
```

A reviewer can replay every adversarial exchange and verify the
outcome reproduces.

## What this changes for ADJ43 and ADJ44

### ADJ43 (TruthfulQA experiment)

Adversarial reading is *the central mechanism* the framework
applies to TruthfulQA. The framework arm doesn't just produce an
answer — it produces an answer plus an adversarial reading, plus
a kickback if they disagree.

The experimental design (preview):
- Baseline: Claude answers TruthfulQA directly
- Framework arm: Claude produces answer + adversary (different
  family) produces alternative reading + judge or 2-model
  agreement check
- Measurement: how many "confidently wrong" answers does the
  framework arm catch vs. miss

The hypothesis: TruthfulQA's wrong answers are *plausible enough*
that a same-family model can be drawn into agreeing. A
cross-family adversary catches them.

### ADJ44 (MYCIN-2026)

Adversarial reading is wired into every LR contribution in the
medications-and-meningitis rulebook. For each `contributes(LR, e,
acs)` clause, a different model produces its own LR estimate; if
they diverge significantly → the rule is marked Uncertainty → the
framework asks for clinical input rather than committing.

This is exactly the discipline differential diagnosis is supposed
to enforce, that MYCIN's certainty factors couldn't.

## Implementation outline

```rust
pub struct AdversarialPipeline {
    primary_extractor: Arc<dyn LlmClient>,
    adversary: Arc<dyn LlmClient>,
    judge: Option<Arc<dyn LlmClient>>,
    policy: AdversarialPolicy,
}

impl AdversarialPipeline {
    pub async fn check_commit_point(
        &self,
        commit_point: CommitPoint,
        original_interpretation: serde_json::Value,
    ) -> AdversarialRecord {
        // 1. Run adversarial reading on the same input
        let adversary_reading = self.adversary.read_adversarially(
            AdversarialContext { ... }
        ).await;

        // 2. Decide based on agreement and policy
        if adversary_reading.agrees_with_original {
            return AdversarialRecord::Agreement { ... };
        }

        // 3. Disagreement: judge or kickback
        match self.policy.resolution_for(commit_point) {
            PolicyResolution::Judge => self.judge_disagreement(...).await,
            PolicyResolution::Kickback => AdversarialRecord::Disagreement {
                resolved_by: ResolvedBy::HumanKickback,
            },
            PolicyResolution::Conservative => self.conservative_choice(...),
        }
    }
}
```

## Status

Draft. Generalizes ADJ05 to the full pipeline. The mechanism is
the same as ADJ05's at each commit point; the contribution is the
unification + the per-commit-point judge protocols + the cost-model
configurability + the audit-trail record format.

Implementation overlaps with ADJ05's existing crate; the natural
move is to extend `adjudication-adversarial` to handle the new
commit points rather than introduce a new crate. The
`AdversarialReader` trait subsumes ADJ05's `FindContradictingReading`
primitive.

## See also

- [ADJ05](ADJ05-adversarial-verifier.md) — the narrower spec this
  generalizes. ADJ42 supersedes ADJ05's commit-point scope but
  preserves its (vendor, family) independence requirement and judge
  protocol.
- [ADJ37](ADJ37-unified-framework-and-rulebook-elicitation-demo.md)
  — surfaced the rulebook-LR commit point as a place adversarial
  reading should apply (the any-delirium-vs-med-induced-delirium
  conclusion-scope mismatch was a single-model's interpretation
  the framework should have challenged).
- [ADJ40](ADJ40-recursive-source-decomposition.md) — citation
  claim-match is one of the six commit points; ADJ42's mechanism
  strengthens ADJ40's `ClaimMatchClient`.
- [ADJ43](ADJ43-truthfulqa-experiment.md) (planned) — uses
  adversarial reading as the central experimental mechanism.
- [ADJ44](ADJ44-mycin-2026.md) (planned) — uses adversarial
  reading on every LR contribution in the meningitis rulebook.
