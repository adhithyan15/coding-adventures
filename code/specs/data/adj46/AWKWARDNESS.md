# ADJ46 — Catalogued awkwardness in encoding ACS rulebook + case via `logic-engine`

Running log. Every time I have to invent an encoding to express something
the ACS rulebook says clearly but the engine's surface doesn't, I note it
here. These are the design inputs to ADJ47 (Adj-Lang).

Format per entry:
- **What the rulebook wants to say**
- **What the engine forces me to write**
- **What primitive the language should have**

---

## A1 — Likelihood ratios are not probabilities

**Wants to say:** `contributes(2.5, pmh(hypertension), acs)` — "patient
having hypertension multiplies the odds of ACS by 2.5."

**Forced to write:**

```rust
// Rule whose head is a synthetic 'contrib' marker keyed by a string id;
// the LR magnitude lives in a side-table because Probability::Value cannot
// hold values > 1.
kb.add_rule(Rule::certain(
    compound("contrib", vec![atom("c_pmh_htn")]),
    vec![BodyLiteral::Pos(compound("pmh", vec![atom("hypertension")]))]
));
lr_table.insert("c_pmh_htn", LrEntry { log_lr: f64::ln(2.5), ... });
```

Two things are mechanically split that semantically belong together:
the *condition* (in the engine) and the *magnitude* (in a side-table).

**Language primitive needed:** `contributes <lr> from <evidence> to
<target>` where `<lr>` is a value of type `LikelihoodRatio` (range
(0, ∞)), not `Probability` (range [0, 1]). The engine should store and
aggregate LR values natively in log-odds space.

---

## A2 — Provenance is not a clause field

**Wants to say:** every `contributes` carries a citation
(`Six AJ et al., Neth Heart J 2008;16(6):191-6.`) that should appear in
the audit document next to the contribution it produced.

**Forced to write:** an entirely separate side-table mapping rule id →
citation string. The engine has no awareness of provenance. The Proof
DAG's `via_rules: Vec<RuleId>` is the only handle; I have to walk that
and join against my side-table in user code.

**Language primitive needed:** `provenance` is a field of every clause,
typed (`Citation { source, locator, trust_tier }`), and surfaces in the
proof DAG without user-side joining. Adj-Lang's clause type should be
`Clause { head, body, probability_or_lr, provenance }` natively.

---

## A3 — Prior is a clause that does not fit the engine's clause types

**Wants to say:** `prior(0.10, acs)` — "the population prior probability
of ACS is 10%."

**Forced to write:** there is no place for this. The engine's `Fact` is a
term + probability where the probability is the *probability of the term
being true in a world*. That's WMC semantics, not Bayesian-prior semantics.
A `prior` for LR aggregation is a *baseline log-odds* the contributions
modulate. The encoding I have to use:

```rust
// Store the prior as a value outside the KB entirely.
let prior_p: f64 = 0.10;
let prior_logodds = (prior_p / (1.0 - prior_p)).ln();
```

**Language primitive needed:** `prior <p> for <atom>` as a first-class
clause that distinguishes "baseline odds" from "world-state probability."

---

## A4 — Joint contributions need a special clause type

**Wants to say:** `contributes_jointly(1.3,
[symptom_quality(pressure_like), associated_symptom(diaphoresis)], acs)`
— "the *combination* of these two pieces of evidence contributes LR 1.3
beyond the product of their individual LRs."

**Forced to write:** a Rule with the joint contribution as head and both
literals as body conditions:

```rust
kb.add_rule(Rule::certain(
    compound("contrib", vec![atom("c_joint_press_diaph")]),
    vec![
        BodyLiteral::Pos(compound("symptom_quality", vec![atom("pressure_like")])),
        BodyLiteral::Pos(compound("associated_symptom", vec![atom("diaphoresis")])),
    ]
));
```

Mechanically works but semantically lies: the engine has no concept that
this is an *interaction term* layered on top of two atomic contributions.
The proof DAG can't distinguish "fired alone" from "fired as part of an
interaction with two atomic contributions also firing."

**Language primitive needed:** `interacts <lr> when [<evidence_set>] for
<target>` syntactically distinct from atomic contributions. Aggregator
should know it's a joint and emit "<atomic_lr1> × <atomic_lr2> × <joint_lr>"
in the audit instead of three opaque numbers.

---

## A5 — Uncertainty markers in the case input have no encoding

**Wants to say:** in the patient case `62yo M, ED for chest discomfort
x 2h. Pressure-like, mild diaphoresis. **No clear precipitator.** PMH:
HTN, smoker.`, the phrase "no clear precipitator" means *the patient's
precipitator is uncertain across {exertional, rest, positional}* — not
that any one of them is true or false.

**Forced to write:** either (a) omit the precipitator atom entirely
(loses information — we know the user said "no clear precipitator"), or
(b) add three competing low-probability facts with hand-chosen
probabilities, or (c) add an `uncertain(precipitator)` atom that no rule
references (loses derivation).

In ADJ46 I'll use (b) with uniform 1/3 weights as the least-bad option,
and flag it.

**Language primitive needed:** `uncertain <atom> over [<domain>]
prior <distribution>` as a first-class clause. The engine should treat
uncertain atoms as candidates for VOI sensitivity analysis automatically.

---

## A6 — The query result is "P(acs)" but I need the proof itself

**Wants to say:** "give me the posterior plus the audit trail."

**Forced to write:** `search(query, kb, SearchMode::EnumerateAll)`
returns `SearchResult::EnumerateAllResult { dag, probability }`. The
`probability` field is the WMC posterior — which is *not* what I want
(LR aggregation gives a different number). I have to throw away
`probability` and re-aggregate from `dag` myself.

**Language primitive needed:** `SearchMode::LRAggregate` per LP19e —
takes the DAG and an LR-table and produces (posterior, ordered audit).

---

## A7 — There's no kickback as a search outcome

**Wants to say:** "the system is not confident enough to commit; here are
the uncertainties whose resolution would change the answer."

**Forced to write:** in the harness, after computing the posterior,
manually compare against a threshold and produce a separate human-readable
output explaining what's missing. Engine has no kickback variant.

**Language primitive needed:** `SearchResult::Kickback {
posterior, required_resolutions: Vec<AtomVOI>, threshold }`.

---

## A8 — Counterfactual queries require KB rebuild

**Wants to say:** `?- counterfactual(precipitator(exertional)=true,
P(acs))` — "what would the posterior be if we knew the patient's
precipitator was exertional?"

**Forced to write:** clone the KB, mutate the case facts, re-run search.
There's no first-class counterfactual primitive.

**Language primitive needed:** `query counterfactual <atom>=<value> for
<target>` returning the perturbed posterior alongside the current one.

---

## A9 — Source disagreement is not expressible

**Wants to say:** the LR for `pressure_like` is 2.5 per Panju 1998 but
2.0 per a different cohort. The audit document should reflect both, not
average them silently.

**Forced to write:** in this ADJ46 demo, I take the AHA/ACC-leaning
number and add a comment. Lossy.

**Language primitive needed:** `contributes <lr> from <evidence> to
<target> via <source>` with multiple `via` clauses per
(evidence, target) pair. Aggregator computes a consensus LR with
explicit weight on each source.

---

## A10 — Rulebook surface syntax is hand-written Rust

**Wants to say:** the rulebook should be readable by a domain expert
(an ED physician) without a Rust compiler in scope.

**Forced to write:** `kb.add_rule(Rule::certain(compound("contrib",
vec![atom("c_pmh_htn")]), vec![BodyLiteral::Pos(compound("pmh",
vec![atom("hypertension")]))]))` — domain-expert-impenetrable.

**Language primitive needed:** the entire surface syntax of Adj-Lang.
The rulebook above should compile to a 2-line declaration.

---

(Awkwardness log will grow as I encode the rest of the rulebook and the
patient case. Updated as ADJ46 progresses.)
