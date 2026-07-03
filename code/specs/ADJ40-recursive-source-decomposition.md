# ADJ40 — Recursive Source Decomposition: Verifying Cited Content, Not Just Citation Existence

> ADJ39 verifies citation *existence*. ADJ40 verifies citation
> *content* — that the source the framework cites actually
> supports the claim the framework's IR attributes to it.
>
> Existence-only verification catches Mata v. Avianca (where the
> case didn't exist). Content verification catches the *next*
> failure mode: cases that exist but whose holdings are
> mischaracterized, statutes that exist but are quoted out of
> context, papers that exist but whose reported numbers are
> misquoted in the brief / paper / DD memo.
>
> The mechanism: when ADJ39 verifies a citation exists, ADJ40
> fetches the full text of the cited source, runs the **same IR
> pipeline** on it, and asks: *does this source's IR contain the
> claim the citing document attributes to it?* If yes, mark
> verified-with-content-match. If no, mark verified-but-claim-
> mismatch and refuse to commit.

## Why the recursive piece matters

A citation can be *real* and still be **wrong**:

- *Varghese v. China Southern Airlines* (Mata v. Avianca) — citation *does not exist* (existence failure, caught by ADJ39).
- *In re Air Crash at Lockerbie* — citation *exists* but the lawyer cited it for a proposition the case doesn't actually hold (claim-mismatch failure, caught by ADJ40).
- *Pfizer v. FDA* — case exists, holding is real, but the brief cites it for an analogy to a different statutory regime where the holding doesn't apply (scope-mismatch failure, caught by ADJ40).

In ADJ38's domain catalog, every domain has a claim-mismatch
failure mode that goes beyond existence:
- **Medicine**: paper exists, but the cited LR value is from a different patient population than the rulebook claims.
- **Finance**: 10-K exists, but the revenue figure cited is from a different fiscal period than the analyst's report claims.
- **Journalism**: study exists, but the reported "50% increase" is misquoted (the actual study reported relative risk on a different exposure variable).
- **Code review**: CWE-89 exists, but the cited code doesn't actually exhibit the named pattern.

The framework's audit-trail guarantee — *"every claim cites a
verified source"* — requires both: the source exists AND the
source actually says what the framework claims.

## Layer position

```
        ADJ01 IR       ADJ39 Verify (existence)
              │                │
              └────────┬───────┘
                       │
                       ▼
                 ┌─────────────┐
                 │  ADJ40      │   ◀── this spec
                 │  Recursive  │
                 │  source     │
                 │  decompose  │
                 └──────┬──────┘
                        │
              ┌─────────┴──────────┐
              ▼                    ▼
       fetch_full_text        IR pipeline
       (from ADJ39 adapter)   (ADJ01-02 applied
                               to the source text)
              │                    │
              └─────────┬──────────┘
                        │
                        ▼
              ┌──────────────────┐
              │ claim-match      │
              │ check (NLI-style)│
              └────────┬─────────┘
                       │
                ┌──────┴──────┐
                ▼             ▼
         verified-     claim_mismatch
         with-content  → kickback to human
         (commit OK)
```

## The mechanism

For each `CitationFact` whose `claimed_proposition` is set
(rulebook-elicited rules attribute specific LRs to specific
papers; legal briefs attribute specific propositions to specific
cases; etc.):

1. **Fetch the source's full text** via the ADJ39 adapter's
   `fetch_full_text(citation)` method. The adapter knows how to
   retrieve the full opinion, paper, or filing.
2. **Run the IR pipeline on the source text** — same
   decomposition, same coverage check, same claim typing. The
   cited source becomes an IRDocument in its own right.
3. **Search the source's IR for a Fact node whose term matches
   the `claimed_proposition`** of the citing CitationFact. This
   is a structured match, not a string match — the source's
   Facts have typed terms and the citing claim has a typed
   proposition.
4. **If found**: the verification status of the original
   CitationFact transitions from `Verified` (existence-only) to
   `Verified` with `match_score` populated and the matching
   source-IR node id recorded.
5. **If not found**: the verification status becomes
   `WrongClaim` with the source's actual claim-set captured for
   the audit trail. The orchestrator refuses to commit any
   inference dependent on this CitationFact and kicks back.

## Recursion termination

Recursive source decomposition is recursive in name only — it
runs at most ONE level deep per citation. The cited source is
processed through the IR pipeline; the source's *own* citations
are not recursively followed (they would be in a fuller
implementation; ADJ40 v0.1 grounds out at depth 1).

This is deliberate:
- Most claim-mismatch failures are caught at depth 1 (the source
  doesn't say what the citing document says it says).
- Recursive-depth-N processing is expensive (each citation
  triggers a full IR pipeline call) and the marginal value drops
  fast.
- v0.2 can lift the depth cap when production data shows
  systematic depth-2 failures.

## The IR pipeline applied to a fetched source

The source's full text is itself an IRDocument. Same Sentence →
Phrase → Claim → TypedComponent decomposition. Same coverage
check. Same typing.

For a court opinion specifically:
- Procedural-history sentences → Facts
- Statement-of-the-case sentences → Facts
- Legal-reasoning sentences → Rule-Facts (these are *holdings*)
- Quotations → Citation-Facts (with their own provenance)
- Dicta → Facts marked with `holding_status: dicta`
- Concurrences and dissents → Facts marked with
  `opinion_part: concurrence` etc.

For a PubMed-indexed scientific paper:
- Abstract claims → Facts marked with `paper_part: abstract`
- Methods statements → Facts marked with `paper_part: methods`
- Results / numerical findings → Facts with quantitative
  TypedComponents
- Discussion → Facts marked with `paper_part: discussion`

The IR pipeline emits these structured Facts. The framework can
then ask: *"does any Rule-Fact in this opinion's IR match the
citing brief's claimed proposition?"* This is a structured
search, not a string match.

## The claim-match check

The match is an NLI-style entailment check between:
- **Citing claim**: `claimed_proposition` from the citing
  CitationFact (a natural-language string in v0.1; a typed
  proposition in v0.2).
- **Source claims**: the set of Rule-Fact and Fact terms in the
  source's IR.

Three flavors of match:

| Strength | Description | Verification status |
|---|---|---|
| **Strong** | The claim is a near-paraphrase of an explicit holding in the source | `Verified { match_score >= 0.9 }` |
| **Partial** | The claim is plausibly supported by reading the source but the wording is different | `Verified { match_score 0.6–0.9 }` |
| **Weak** | The claim mentions terms in the source but the proposition is not stated | `WrongClaim` (treated as failed) |

The NLI-style check is itself an LLM call — the framework's
internal `entail(claim, source_text)` primitive. It returns a
score plus a justification. The justification is recorded in
the audit trail.

**The framework refuses to commit on `Weak` matches** — those
are the "the cited paper exists but doesn't actually say what
you claim" cases.

## Citing-claim provenance — input vs. rulebook

A `CitationFact` can come from two sources:

- **Citing brief / case (input IR)**: the lawyer / analyst /
  journalist cites a source. The `claimed_proposition` is from
  the citing document's text.
- **LLM-elicited rulebook (rulebook IR)**: per ADJ37, the LLM
  produced a natural-language rulebook that cites sources. The
  `claimed_proposition` is from the rulebook's text.

Both go through the same ADJ40 verification. **A rulebook
hallucinating a fact about a real paper is treated the same as
a brief hallucinating a fact about a real case.** Symmetry.

## The orchestrator integration

After ADJ39 verifies existence:

```python
async def verify_citations_with_content(ir):
    for fact in ir.citation_facts():
        if fact.verification == Pending:
            # Run ADJ39 existence check
            fact.verification = await existence_verifier.verify(fact)

        # ADJ40: if existence passed and we have a claimed
        # proposition, verify content match
        if (fact.verification.is_verified()
            and fact.claimed_proposition is not None):

            source_text = await existence_verifier.fetch_full_text(fact)
            if source_text is None:
                # Cannot fetch full text — keep existence-only verification
                # but record the limitation
                fact.verification.add_note("content_match_not_attempted_no_text")
                continue

            source_ir = run_ir_pipeline(source_text, source_id=fact.id)
            match = await claim_match_check(
                claim=fact.claimed_proposition,
                source_ir=source_ir,
            )

            if match.strength == "Weak":
                fact.verification = VerificationStatus.WrongClaim {
                    detail: match.justification,
                    actual_content_summary: source_ir.summary(),
                }
                # orchestrator will refuse to commit on this
```

## Cost considerations

Recursive content verification is **expensive**:
- One full IR pipeline call per cited source (potentially many
  bytes — court opinions can be 50+ pages; papers can be 20+
  pages).
- One LLM entail call per claimed-proposition match.

ADJ40 uses the same caching layer as ADJ39: source-IR results
keyed on `(source_id, IR-pipeline-version-hash)`. Re-runs are
fast.

Cost budgets:
- ADJ40 defaults to checking only `claimed_proposition`-bearing
  CitationFacts (existence-only is cheap; content-match is
  expensive).
- Per-citation cap on source text fetched (e.g., 100k bytes).
- Per-adjudication cap on total content-match LLM calls.

The framework's free-tier guarantee: content-match runs only
when a `claimed_proposition` is set and full text is fetchable.
For citations without `claimed_proposition` (e.g., "see also
ABC v. XYZ"), ADJ40 doesn't run content-match — the citation is
recorded as supporting authority but not for any specific claim.

## The Mata v. Avianca **content** scenario

What if the Varghese case had been *real* but the lawyer
mischaracterized its holding? With ADJ39 + ADJ40 together:

```
Citation: Varghese v. China Southern Airlines, 925 F.3d 1339 (2d Cir. 2019)
Claimed proposition: "statute-of-limitations defenses are
                      non-waivable in diversity actions"

ADJ39: existence check
  → CourtListener returns: [hypothetically] "Yes, Varghese v.
    China Southern Airlines, 925 F.3d 1339 (2019)" exists
  → VerificationStatus::Verified (existence)

ADJ40: content check (because claimed_proposition is set)
  → fetch_full_text returns the opinion
  → IR pipeline runs on the opinion → produces 247 typed Facts
    including ~30 Rule-Facts (holdings)
  → claim_match_check searches Rule-Facts for "statute-of-
    limitations defenses are non-waivable in diversity actions"
  → No matching Rule-Fact found. The opinion is about
    Montreal-Convention-2-year limits, not statute-of-
    limitations non-waivability.
  → VerificationStatus updated to WrongClaim
  → Orchestrator refuses to commit

The lawyer thought citation existed → it does.
The lawyer thought the holding supported their proposition →
   ADJ40 verifies; finds no support; refuses to commit.
```

**The brief still doesn't ship.** Both verification layers
combined catch both Mata v. Avianca *and* the "real case,
wrong holding" failure mode.

## What ADJ40 v0.1 ships (and what's deferred)

**v0.1 (this spec):**
- The recursive-source-decomposition mechanism — fetch + IR +
  claim-match
- The 3 match strengths (Strong, Partial, Weak)
- Integration with ADJ39's CitationVerifier (uses fetch_full_text)
- Depth-1 recursion (don't follow the source's own citations)
- Cost budgets

**Deferred to v0.2:**
- Multi-level recursion when v0.1 production data shows it's
  needed
- Typed-proposition matching (not just NLI on natural language)
- Holding-extraction heuristics specific to court opinion
  format

**Deferred to follow-up:**
- Paper-section-specific weighting (a result-section claim is
  stronger evidence than a discussion-section claim)
- Multi-source corroboration (if multiple sources support the
  same claim, weight it higher)

## Failure-mode coverage delta (ADJ39 → ADJ39 + ADJ40)

Updated from ADJ39's matrix:

| Citation kind | Existence (ADJ39) | Wrong content (ADJ40) |
|---|---|---|
| CaseLaw | ✓ | ✓ (NLI on opinion text) |
| Statute | ✓ | ✓ (NLI on statutory text) |
| Article | ✓ | ✓ (NLI on abstract + results) |
| SecFiling | ✓ | ✓ (filing item-level) |
| Vulnerability | ✓ | ✓ (database content) |
| Patent | ✓ | ✓ (claims text) |

**Combined, ADJ39 + ADJ40 give the Mata v. Avianca guarantee in
full generality**: not just "the cited case must exist" but "the
cited case must say what you say it says."

## Implementation outline

```rust
pub struct RecursiveSourceVerifier {
    existence_verifier: Arc<dyn CitationVerifier>,
    claim_match_client: Arc<dyn ClaimMatchClient>,
    source_ir_cache: Arc<dyn IrCache>,
}

#[async_trait]
pub trait ClaimMatchClient: Send + Sync {
    async fn match_claim(
        &self,
        claim: &str,
        source_ir: &IRDocument,
    ) -> MatchResult;
}

pub struct MatchResult {
    pub strength: MatchStrength,
    pub matched_node_ids: Vec<NodeId>,  // which Facts in source IR matched
    pub justification: String,           // why this strength
}

pub enum MatchStrength {
    Strong,
    Partial,
    Weak,
}
```

The `ClaimMatchClient` is the LLM-driven NLI primitive. Its
implementation calls an LLM with a structured prompt:

```
SYSTEM: You evaluate whether a source supports a citing claim.
        Given:
          - A CITING CLAIM: <claim>
          - A SOURCE IR (list of typed Facts):
            <source_ir.facts>
        Output:
          - A strength (Strong/Partial/Weak)
          - The list of source-Fact IDs that support the claim
          - A justification

        Strong = the source contains a Rule-Fact whose term is
                 a near-paraphrase of the citing claim.
        Partial = source's Facts imply the claim but don't state
                  it directly.
        Weak = source's Facts don't contain or imply the claim.
```

The framework's primary trust signal for the match: the LLM
producing the same answer across multiple invocations with the
same input. ADJ05 adversarial-verifier-style cross-model check
can be applied — two different models perform the match, agreement
required.

## Status

Draft. This is Gap 2 from ADJ38; the natural complement to
ADJ39. Implementation work follows the same staged approach as
ADJ39: foundational crate + adapter integration + CLI tooling.

## See also

- [ADJ39](ADJ39-citation-verification-infrastructure.md) — the
  existence-verification spec ADJ40 builds on.
- [ADJ38](ADJ38-cross-domain-framework-validation.md) — the
  cross-domain analysis that identified content verification as
  the higher-impact half of the Mata v. Avianca guarantee.
- [ADJ37](ADJ37-unified-framework-and-rulebook-elicitation-demo.md)
  — the rulebook-elicitation spec where rulebook-cited papers
  were flagged for content verification but not actually verified.
  ADJ40 makes those checks real.
- [ADJ01](ADJ01-adjudication-ir-grammar.md) — the IR pipeline
  ADJ40 recursively applies to fetched sources.
- [ADJ05](ADJ05-adversarial-verifier.md) — the cross-model
  agreement check that can strengthen ClaimMatchClient
  reliability.
