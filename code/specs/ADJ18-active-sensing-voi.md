# ADJ18 — Active Sensing via Value-of-Information on the Proof DAG

> The kick-back spec. When the framework's verdict is sensitive to
> an uncertain atom whose resolution would shift the answer beyond
> a threshold, refuse to commit and ask for clarification. Closes
> the loop the user named explicitly: *"a case like matches should
> be kicked back for more clarification."*

## Overview

By ADJ14 + LP19e the framework produces a posterior probability
over each conclusion, derived from a prior plus a list of named
LR contributions. By ADJ15 the proof DAG records every contribution
as a typed step in the audit trail. By ADJ16 the proof DAG renders
as defensible prose. Those three pieces give the framework a
**defensible verdict**.

But a defensible verdict is not always the right output. In the
matches case ([ADJ12 v2 §"The scale curve"](ADJ12-small-model-benchmarks.md)),
the source text is genuinely ambiguous; the rulebook's verdict
depends on a fact the source did not commit to (what *kind* of
match — safety, strike-anywhere, lit, unlit?); and the right
behavior is *not* to issue a verdict at all but to **ask for the
missing fact**.

This is the **active sensing** problem. The framework needs:

1. A formal measure of how much an uncertain atom matters to the
   verdict on a given query (the **value of information**, VOI).
2. A decision rule for when to commit vs. kick back.
3. A mechanism for surfacing the highest-VOI atom as a structured
   clarification request to ADJ06.

This spec defines all three.

## The intuition, in one paragraph

A query's verdict is a function of observed evidence. Some pieces
of unobserved evidence, if observed, would shift the verdict more
than others. The framework can compute *exactly how much* each
unobserved atom would shift the posterior, because the LR
aggregation algorithm (LP19e) is closed-form. The atom whose
resolution would change the verdict the most is the one to ask
about. If no atom's resolution would change the verdict beyond a
threshold, commit. Otherwise, kick back with the highest-VOI
question.

## Layer position

```
   ADJ14   probabilistic IR semantics
        │
        ▼
   LP19e   LR aggregation engine
        │
        ▼
   ADJ15   proof DAG in audit trail
        │
        ▼
   ADJ18   THIS SPEC — VOI computation + kick-back decision
        │
        ▼
   ADJ06   clarification dialogue (consumes ADJ18's structured questions)
```

ADJ18 depends on ADJ14 and LP19e (without LR aggregation there's no
closed-form posterior to differentiate). It produces structured
clarification requests that ADJ06 already knows how to issue.

## The math

Given a query `Q` and the current observed evidence `E`, the
framework computes a posterior `P(Q | E)` via LP19e. For an
unobserved atom `a` with two possible resolutions (`a` observed
true, `a` observed false), define:

```text
P_+ = P(Q | E ∪ {a observed true})
P_− = P(Q | E ∪ {a observed false})
P_0 = P(Q | E)                       (current posterior)
```

The **value of information** of resolving `a` is the *expected
verdict shift* if we were to resolve it:

```text
VOI(a) = π_a · |verdict(P_+) ≠ verdict(P_0)|
       + (1 − π_a) · |verdict(P_−) ≠ verdict(P_0)|
```

where `π_a` is the framework's prior probability that `a` would
resolve to true given the current evidence (a marginalization over
other contributors), and `|verdict(p) ≠ verdict(P_0)|` is `1` if
the resolution would change the discrete verdict (above/below
threshold) and `0` otherwise.

For continuous (probability-shifting) VOI, the same expression
holds with `|verdict(p) ≠ verdict(P_0)|` replaced by the absolute
probability shift `|p − P_0|`:

```text
VOI_continuous(a) = π_a · |P_+ − P_0| + (1 − π_a) · |P_− − P_0|
```

Both measures are O(1) to compute per atom because LP19e's
inference is closed-form log-odds arithmetic. The framework
evaluates VOI for every atom that appears as the evidence term of
a `contributes(...)` clause for any conclusion in the query set.

### Worked example — the matches case

Source: `"1 carry-on bag, matches."`

Rulebook (illustrative; not the actual TSA regulations, but
plausible):

```text
prior(0.05, prohibited)
contributes(LR = 20.0, lit_match,           prohibited)
contributes(LR = 10.0, strike_anywhere_match, prohibited)
contributes(LR = 0.5,  safety_match,        prohibited)
```

The extractor produces an IR with `Fact(carry_on_bag(1))` and
`Uncertainty(match_type, confidence: 0.5)` — the framework knows
matches are present, but the type is undetermined.

In LP19e under the current evidence:

- No evidence committed on `lit_match`, `strike_anywhere_match`,
  or `safety_match`.
- The posterior on `prohibited` is the prior: `P(prohibited | E) =
  0.05`.

The framework computes VOI for each unobserved atom:

```text
VOI(lit_match):
  Observed true:  posterior_logit = log(0.05/0.95) + log(20.0) = +0.054
                   P_+ = sigmoid(+0.054) = 0.513
  Observed false: P_− = 0.05 (no contribution applied)
  π_a ≈ 0.10 (matches in the wild rarely lit when carried)
  VOI(lit_match) = 0.10 · 0.46 + 0.90 · 0.00 = 0.046

VOI(strike_anywhere_match):
  Observed true:  P_+ = sigmoid(log(0.05/0.95) + log(10.0)) = 0.345
  Observed false: P_− = 0.05
  π_a ≈ 0.30
  VOI = 0.30 · 0.30 + 0.70 · 0.00 = 0.090

VOI(safety_match):
  Observed true:  P_+ = sigmoid(log(0.05/0.95) + log(0.5)) = 0.026
  Observed false: P_− = 0.05
  π_a ≈ 0.60
  VOI = 0.60 · 0.024 + 0.40 · 0.00 = 0.014
```

The dominant VOI is on `strike_anywhere_match` (0.090) — about 2×
the next-highest. The framework's kick-back is:

> **The verdict on `prohibited` depends on the type of match in
> the carry-on. The source does not specify the match type. Most
> consequential to clarify: are the matches *strike-anywhere* or
> *safety* matches?**

This is exactly the response a TSA agent would give. The framework
arrives at it by closed-form computation, not heuristic.

## The decision rule

Define two thresholds:

```text
voi_threshold_kickback:    default 0.10   (resolve before committing)
voi_threshold_warn:        default 0.03   (commit but warn)
```

Decision logic per query:

```python
def decide(query, kb, observed_evidence):
    posterior, dag = lr_aggregate(query, kb, observed_evidence)
    candidates = atoms_with_voi(query, kb, observed_evidence)
    top = max(candidates, key=lambda a: a.voi) if candidates else None

    if top and top.voi >= voi_threshold_kickback:
        return Verdict.KickBack(
            question=structured_question(top, query, kb),
            current_posterior=posterior,
            voi=top.voi,
            dag=dag,
        )

    if top and top.voi >= voi_threshold_warn:
        return Verdict.CommittedWithWarning(
            answer=posterior,
            warning=f"resolving {top.atom} could shift verdict by {top.voi:.2f}",
            dag=dag,
        )

    return Verdict.Committed(answer=posterior, dag=dag)
```

Three terminal states:
1. **Committed**: no atom's resolution would shift the verdict
   beyond the warn threshold. Framework returns the posterior.
2. **Committed with warning**: at least one atom would shift the
   verdict modestly (`warn ≤ voi < kickback`). The framework still
   returns a posterior, but the audit trail records the warning.
3. **Kick-back**: at least one atom would shift the verdict
   beyond the kick-back threshold. The framework refuses to
   commit and surfaces a structured question to ADJ06.

The thresholds are configurable per deployment. High-stakes
domains (medical, legal) lower them; low-stakes domains (consumer
chatbot) raise them.

## The structured kick-back

A kick-back is not free-form prose — it is a typed object ADJ06
consumes:

```rust
pub struct KickBack {
    /// The query that triggered the kick-back.
    pub query: Term,

    /// The atom whose resolution would most-shift the verdict.
    pub focal_atom: Term,

    /// The structured question, in the same vocabulary as the
    /// rulebook's evidence terms. Multiple-choice when the atom's
    /// resolution is one of a finite set; open-ended otherwise.
    pub question: ClarificationQuestion,

    /// Current posterior under existing evidence (would be issued
    /// if the framework didn't kick back).
    pub current_posterior: f64,

    /// Quantified VOI for this atom.
    pub voi: f64,

    /// All other atoms with VOI ≥ warn threshold, for transparency.
    pub other_voi_atoms: Vec<(Term, f64)>,

    /// The proof DAG up to the kick-back point.
    pub dag: ProofDAG,
}

pub enum ClarificationQuestion {
    /// "Is X true or false?" — applies to atoms with binary
    /// resolution.
    Binary { atom: Term, prompt: String },

    /// "Which of these is the case?" — applies to atoms that are
    /// the target of a mutually-exclusive set of `contributes(...)`
    /// clauses (e.g., match type).
    MultipleChoice {
        family: String,             // e.g., "match_type"
        options: Vec<Term>,         // e.g., [safety, strike_anywhere, lit]
        prompt: String,
    },

    /// "Please specify X" — open-ended; only used as a last resort.
    OpenEnded { atom: Term, prompt: String },
}
```

For the matches case, the kick-back is:

```text
KickBack {
    query: prohibited,
    focal_atom: match_type,
    question: MultipleChoice {
        family: "match_type",
        options: [safety_match, strike_anywhere_match, lit_match],
        prompt: "What type of matches are these?",
    },
    current_posterior: 0.05,
    voi: 0.090,
    other_voi_atoms: [(lit_match, 0.046), (safety_match, 0.014)],
    dag: <prior + no contributions yet>,
}
```

ADJ06 receives this object and surfaces the prompt to whichever
upstream consumer is configured: a user-facing UI, a domain-expert
review queue, an LLM-based clarification dialogue, or a second
model with a "closer reading" prompt.

## Integration with the four-rung escalation ladder

ADJ06 today escalates clarification requests across four rungs:

```text
Rung 1: Same model re-prompted with structured violation.
Rung 2: Different model from different family, same prompt shape.
Rung 3: Human reviewer.
Rung 4: Abort with audit-trail record.
```

ADJ18's kick-back enters at Rung 1 by default and escalates the
same way if Rung 1 cannot resolve the question. The cost-vs-VOI
math justifies the escalation:

- Rung 1 (same model, ~$0.001): try first.
- Rung 2 (different model, ~$0.002): try if Rung 1 hedges or
  contradicts.
- Rung 3 (human, ~$10): try if VOI ≥ a high human-cost threshold
  and Rung 1+2 didn't resolve.
- Rung 4 (abort): if all rungs fail and VOI ≥ threshold, refuse
  the verdict.

The framework can compute the expected utility of each rung by
combining VOI (in verdict shift units) with the deployment's cost
table (in dollars or seconds or whatever the right unit is). This
is **principled cost-of-information sensing** — the framework
spends the minimum to resolve the maximum-shifting uncertainty.

## Mode interaction with LP19e

ADJ18's VOI computation requires `LP19e::lr_aggregate_under(query,
evidence_overlay, kb)` — an LP19e API extension that runs inference
under a hypothetical evidence set without mutating the KB. The
extension is mechanical (LP19e is already closed-form; adding an
overlay parameter is a small wrapper):

```rust
impl KnowledgeBase {
    /// Run LR aggregation as if `overlay_observed` and
    /// `overlay_not_observed` were the only differences from the
    /// current observed evidence. Used by ADJ18 for VOI hypotheticals.
    pub fn lr_aggregate_under(
        &self,
        query: &Term,
        overlay_observed: &[Term],
        overlay_not_observed: &[Term],
    ) -> LRAggregateResult;
}
```

Cost: O(n) per call; one call per atom × 2 (true / false hypotheses).
For a query with ~20 contributors, a full VOI scan is ~40 closed-form
arithmetic operations. **Negligible** compared to the LLM call cost
of either the original extraction or the kick-back.

## Where π_a comes from

The `π_a` (prior probability that an unobserved atom would resolve
to true) in the VOI computation is itself an open modeling
question:

1. **Uniform**: π_a = 0.5 for all atoms. Defensible default; doesn't
   require domain knowledge.
2. **Rulebook-declared**: deploy `prior(p, a)` for each evidence
   atom that the rulebook author wants weighted. Adds rulebook
   work but produces calibrated VOIs.
3. **Empirically learned**: from labelled outcome data, fit π_a per
   atom. Production deployments with sufficient data.

The framework's v0.1 specifies the uniform default with an
explicit override path. Calibration is the modeler's responsibility,
same as for LR aggregation.

## Open questions

1. **VOI on joint contributions.** A `contributes_jointly(LR_extra,
   [a, b, c], conclusion)` clause has cumulative VOI that depends
   on the joint resolution probability of all evidence atoms in the
   set. v0.1 computes per-atom VOI ignoring joint terms; v0.2
   should compute the joint set's VOI as a single unit when the set
   is small.
2. **Counterfactual sensitivity.** "Would the verdict change if the
   prior were different?" is a related question — not VOI on
   evidence but VOI on rule weight. Useful for surfacing
   miscalibrated rulebooks. Deferred to future ADJ18b.
3. **Multi-conclusion VOI.** A clinical query asks for posteriors
   on multiple candidate diagnoses simultaneously. The dominant
   VOI atom may differ per conclusion. v0.1 computes per-query VOI;
   a batched multi-query VOI is future work.
4. **Threshold calibration.** The default thresholds (kickback 0.10,
   warn 0.03) are *guesses*. Empirical calibration on real
   deployment data — when do humans want to be asked? — is needed
   before these defaults are claimed to be appropriate.

## Limitations

1. **VOI is sensitive to π_a.** A miscalibrated `π_a` produces a
   miscalibrated VOI. The framework records `π_a` per computation
   in the audit trail so reviewers can see what assumptions
   produced the kick-back decision.
2. **Single-atom focus.** v0.1 surfaces the single highest-VOI
   atom in each kick-back. When multiple atoms have nearly-equal
   VOI, surfacing the top one is arbitrary; the others appear in
   `other_voi_atoms` but the framework doesn't ask about them.
3. **Continuous VOI is a probability shift, not a utility shift.**
   For decision-theoretic queries ("should we order this test?")
   the right object is expected utility shift, not probability
   shift. The framework's v0.1 specifies probability shift; a
   future ADJ-decision-theoretic spec will extend to utility.
4. **Open-world VOI is heuristic.** The framework can only ask
   about atoms it knows might matter — i.e., atoms that appear as
   evidence terms in some `contributes(...)` clause. Unknown
   relevant evidence remains unknown.

## The matches case, end-to-end

Putting it all together for the canonical fixture:

```text
Source: "1 carry-on bag, matches."
Rulebook (illustrative):
  prior(0.05, prohibited)
  contributes(LR=20.0, lit_match, prohibited)
  contributes(LR=10.0, strike_anywhere_match, prohibited)
  contributes(LR=0.5,  safety_match, prohibited)

Extractor IR:
  Fact(carry_on_bag(1))
  Uncertainty(match_type, confidence: 0.5)
  Query(prohibited)

LP19e under current evidence:
  posterior(prohibited | observed) = prior = 0.05

ADJ18 VOI scan (default thresholds):
  VOI(strike_anywhere_match) = 0.090  ← > 0.10? No, but close.
  VOI(lit_match)             = 0.046
  VOI(safety_match)          = 0.014

Outcome (with default thresholds):
  → Committed with warning  (max VOI = 0.090 in [warn, kickback))
  → Posterior 5% issued; warning records the focal atom.

Outcome (with high-stakes thresholds, e.g., kickback = 0.05):
  → KickBack {
      focal_atom: strike_anywhere_match,
      question: MultipleChoice {
        family: "match_type",
        options: [safety_match, strike_anywhere_match, lit_match],
        prompt: "What type of matches are these?",
      },
      voi: 0.090,
    }
  → ADJ06 surfaces the question; verdict deferred until resolved.

After clarification (say: "safety matches"):
  Re-run LP19e with evidence = {carry_on_bag(1), safety_match}.
  posterior(prohibited | observed) = sigmoid(log(0.05/0.95) + log(0.5))
                                   = 0.026
  → Committed: ~2.6% chance prohibited.
  → Verdict: COMPLIANT with audit trail showing prior, contribution,
    and the clarification turn.
```

The same fixture that triggered the 70B confident-hallucination
in ADJ12 v2 — "Matches are prohibited.", verdict NON-COMPLIANT —
gets handled correctly by the LR-aggregated framework with active
sensing: either committed with a low posterior on prohibition, or
kicked back for the missing fact (match type), or, after
clarification, committed with a calibrated 2.6% prohibition
probability and a full derivation. The right answer arrives by
construction, not by hoping the model gets the right answer.

## Status

Draft. Implementation depends on LP19e (the engine sub-spec for LR
aggregation) and ADJ15 (typed proof DAG in audit trail). The
VOI math is closed-form and self-contained; the kick-back surface
plugs into ADJ06's existing dialogue machinery.

## Where to read next

- [ADJ14](ADJ14-probabilistic-ir-semantics.md) — the LR-aggregation
  semantics this spec computes VOI on.
- [LP19e](LP19e-likelihood-ratio-aggregation.md) — the engine
  inference primitive `lr_aggregate_under` extends.
- [ADJ15](ADJ15-lowering-map-and-proof-dag.md) — the typed proof
  DAG that ADJ18's kick-back returns.
- [ADJ06](ADJ06-clarification-dialogue.md) — the dialogue machinery
  that consumes ADJ18's structured questions.
- [ADJ12](ADJ12-small-model-benchmarks.md) §"The scale curve" — the
  empirical motivation for the matches case.
- [ADJ19](ADJ19-expert-systems-historical-analysis.md) §9 — the
  framework's response to confident-hallucination-at-scale, of
  which ADJ18 is the proactive (kick-back-before-commit) half;
  ADJ04 + ADJ05 are the reactive (catch-after-the-fact) half.
