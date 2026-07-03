# Paper 1 · E3 — domain-expansion plan (the broader cross-domain matrix)

> **Work item W11.** The user's call: *"we will probably have to do a lot more different-domains
> experiments."* This plans the scale-up of E3 from the current ~9 mixed runs to a uniform, N≥15
> matrix — with the measurement discipline ([`MEASUREMENT-VALIDITY-AUDIT.md`](MEASUREMENT-VALIDITY-AUDIT.md),
> [`PAPER1-methods-protocol.md`](PAPER1-methods-protocol.md)) baked in **from day one**.
> Consolidation of what exists: [`PAPER1-E3-crossdomain-consolidation.md`](PAPER1-E3-crossdomain-consolidation.md).

## 0. Precondition (non-negotiable)

**Do not add domains under a broken metric.** The corpus audit found the defensibility metric was
format-confounded; new domains must use the **corrected** rubric (locus-exposure), **format
normalization**, **≥2 judges**, and **deterministic/style-invariant accuracy scoring**, or they just
manufacture more confounded numbers at scale. W11 runs *after* the W5 gate sets judge policy.

## 1. What the breadth claim needs to become statistically real

The current E3 evidence is **9 domains, mixed protocols** — enough for an existence proof ("same
machinery, many domains"), not for a quantified claim. To support *"the framework lifts defensibility
across the space of adjudicative work,"* we need:
- a **principled domain sample** (not a convenience sample) that spans the axes that plausibly modulate
  the effect;
- **uniform protocol** across every domain (same arms, same corrected metric, same controls);
- enough domains (target **N ≥ 15**) to report an effect **with a domain-level distribution**, not a
  per-domain anecdote.

## 2. Domain-selection axes (sample the space, don't cherry-pick)

Adjudicative knowledge work = *apply criteria to evidence for a defensible judgment.* Stratify
candidate domains along axes that should modulate where the framework helps vs. is neutral (the ADJ99
texture: lookup/rule-dense → framework wins; self-contained derivation → neutral):

| axis | low end | high end |
|---|---|---|
| rule structure | precedent/analogy-dense (case law) | rule/threshold-dense (tax, building code) |
| evidence type | qualitative narrative | quantitative thresholds |
| rulebook availability | tacit / proprietary | public authoritative text (statute, guideline) |
| stakes | low (content tagging) | high (clinical, legal eligibility) |
| answer checkability | free-form | exact/numeric (deterministically gradable) |

Pick domains to **fill the cells**, including deliberately *hard-for-the-framework* ones (qualitative,
tacit-rulebook) so the breadth claim is honest about where it's neutral.

## 3. Candidate domain bank (existing ✓ + new)

| domain | rulebook source | axis fit | status |
|---|---|---|---|
| Clinical triage (ACS, meningitis) | guidelines | high-stakes, quantitative | ✓ ADJ44/48/50 |
| M&A deal completion | deal criteria | finance, rule-dense | ✓ ADJ49 |
| Aviation/FSIA jurisdiction | case law | precedent-dense | ✓ ADJ38 |
| Naturalization eligibility (8 USC 1427) | statute | rule-dense, public | ✓ ADJ71 |
| **Tax determination** (sales/use, residency) | tax code | quantitative, public, checkable | new |
| **Insurance claim adjudication** | policy + regs | rule-dense, mid-stakes | new |
| **Benefits eligibility** (SSDI / SNAP) | program rules | rule-dense, high-stakes | new |
| **Building-code compliance** | IBC/local code | quantitative thresholds | new |
| **Patent claim / 101 eligibility** | statute + MPEP | precedent+rule mix | new |
| **Regulatory filing compliance** (SEC/FDA) | regulation | rule-dense, public | new |
| **Credit/loan underwriting** | lending criteria | quantitative, mid-stakes | new |
| **Academic peer-review accept/reject** | venue rubric | qualitative (hard case) | new |
| **Content-moderation policy** | policy doc | qualitative, tacit-ish (hard case) | new |
| **Grant/scholarship eligibility** | program rubric | rule-dense | new |
| **Environmental permitting** | statute + thresholds | quantitative | new |
| **Standardized-rubric grading** | rubric | qualitative, checkable-ish | new |

Target: ground **≥6 new** domains to reach N≥15 with the uniform protocol; bias the new set toward
**public, checkable rulebooks** (deterministic accuracy) and include ≥2 **hard** qualitative domains.

## 4. The reusable per-domain harness (no per-domain code)

The whole point: a new domain is **data, not code**. One harness, parameterized by `(corpus, rulebook,
cases)`:
1. **Ground** the rulebook from the public source (byte-provenanced; citations flagged-for-verification).
2. **Compile** rulebook → executable (adj-lang / CAS library, per ADJ48/71).
3. **Run** held-out cases through both arms (bare prose vs framework).
4. **Score** with the uniform corrected protocol (below).
5. **Emit** `code/specs/data/e3-domains/<domain>/` (corpus, rulebook, cases, raw, FINDINGS) — same
   layout every domain, so cross-domain aggregation is mechanical.

A new domain that needs code changes is a **finding** (a gap in the machinery), logged — not patched
silently.

## 5. Uniform scoring protocol (locked)
- **Defensibility:** corrected locus-exposure rubric, format-normalized, ≥2 judges, report inter-judge
  agreement (per W5 policy).
- **Accuracy:** deterministic / style-invariant from saved raw + gold; LLM accuracy judge only as a
  flagged approximation.
- **Per-domain record:** `ran_with_no_code_change?`, framework-vs-plain defensibility (Δ + CI),
  accuracy (Δ), abstention/kickback rate, failure-mode observed (ADJ56 taxonomy: grounded
  extrapolation / ungrounded over-penalization / commitment-gap).
- **Contamination:** held-out / less-contaminated cases; never score the open-book framework
  closed-book on recall.

## 6. The headline E3 figure (what scale buys)
A domain-level distribution: framework-minus-plain defensibility across N≥15 domains, with CIs, broken
out by the §2 axes — showing **where the lift is large (rule-dense, public-rulebook, quantitative)**,
**where it's neutral (self-contained / tacit-rulebook)**, and the **honest failure-mode rate**. That
converts "it works in several domains" into "here is the *shape* of where byte-provenance helps."

## 7. Sequencing & cost
1. Gate on **W5** (judge policy) and on the Tier-1 rescores (corrected metric in hand).
2. Build the parameterized harness once (extends the ADJ48/71 path).
3. Ground new domains in waves of ~3; each wave is one PR (`e3-domains/<domain>/`).
4. Aggregate when N≥15; report the distribution + axis breakdown.
Cost scales with domains × cases × (2 arms) × (≥2 judges) — front-load **public-checkable** domains so
accuracy is deterministic and only defensibility needs judges.

## 8. Open questions for the user
- **How many domains / how high to push N** (15 is the floor for a distribution; 20–25 makes the axis
  breakdown crisp but multiplies cost).
- **Domain priorities** — which of §3's new domains matter most to the paper's audience (legal/clinical
  lead for credibility; tax/benefits lead for "checkable + high-stakes").
- Whether any domain should be **adversarially chosen to fail** (a tacit-rulebook domain) to bound the
  claim — recommended for honesty.
