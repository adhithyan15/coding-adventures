# ADJ41 — Decomposed Source-IR Store: Verification as Amortized Knowledge-Graph Construction

> ADJ40 framed recursive source decomposition as a per-verification
> cost. That framing was wrong. **A source's IR decomposition is a
> one-time cost, amortizable across every future citation of that
> source, by every user, forever.** A paper published in 2000 will
> not change; once decomposed, its typed IR is permanent.
>
> The framework is therefore not "we make verification possible at
> high cost." It is **"we build a cumulative knowledge graph of
> decomposed authoritative sources, amortized across every framework
> user, that drives per-verification cost toward zero
> asymptotically."**
>
> This is a stronger architectural claim than the per-query-cost
> framing and matches how human knowledge work actually accumulates
> — a law firm's research library has marked-up versions of every
> case they've cited; a clinician's reference shelf has annotated
> papers; an expert's working memory is a decomposed knowledge graph
> of their domain.
>
> ADJ41 specifies the source-IR store, its interface, its versioning
> model, its sharing/federation strategy, and the cost-model reframe.
> Also corrects ADJ40's per-query framing.

## What ADJ40 had wrong

ADJ40's §"Cost considerations" wrote:

> "Recursive content verification is **expensive**: one full IR
> pipeline call per cited source (potentially many bytes — court
> opinions can be 50+ pages; papers can be 20+ pages). One LLM
> entail call per claimed-proposition match."

This treated *every* source-text decomposition as a per-verification
cost. The reframe: **source decomposition is per-source, not
per-citation.** Once *Pope NEJM 2000* has been decomposed into a
typed IR, every subsequent rulebook that cites it — by every user
of every deployment of the framework — pays only the lookup cost
plus an NLI call against the pre-existing IR.

## The right cost model

Two distinct costs per citation:

| Cost | When | Magnitude |
|---|---|---|
| Source-IR decomposition | First-ever citation of a given source | Full IR pipeline call on source text (potentially substantial — 50+ pages for a court opinion) |
| Claim-match | Every citation of an already-indexed source | One lookup + one NLI call (small) |

In the steady state of a populated store: **all source decomposition
is amortized to zero per query.** Verification cost approaches one
NLI call per citation, asymptotically.

The cold-start cost can be amortized across users via shared/public
stores (see §"Federation and sharing" below).

## The store

```rust
/// Persistent, content-addressed store of decomposed source IRs.
///
/// One entry per (source_identity, source_version). The framework
/// queries this store before invoking the IR pipeline on any
/// source: if the source is already indexed, return its IR;
/// otherwise, run the IR pipeline and insert the result.
#[async_trait]
pub trait SourceIrStore: Send + Sync {
    /// Look up the source's pre-decomposed IR if available.
    async fn get(
        &self,
        identity: &SourceIdentity,
        version: Option<&SourceVersion>,
    ) -> Option<DecomposedSource>;

    /// Persist a newly-decomposed source. Returns the
    /// content-addressed key for retrieval.
    async fn put(
        &self,
        identity: SourceIdentity,
        version: SourceVersion,
        ir: IRDocument,
        provenance: DecompositionProvenance,
    ) -> Result<SourceKey, StoreError>;

    /// Query versions available for a given source.
    async fn list_versions(
        &self,
        identity: &SourceIdentity,
    ) -> Vec<SourceVersion>;

    /// Whether this store is the local cache (writable) or
    /// a remote/shared store (read-only).
    fn locality(&self) -> StoreLocality;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceIdentity {
    pub citation_kind: CitationKind,       // from ADJ39
    pub canonical_id: String,              // best stable identifier:
                                            //   DOI for papers,
                                            //   reporter+vol+page for cases,
                                            //   accession for SEC filings,
                                            //   etc.
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceVersion {
    /// Version designation appropriate to the source type:
    ///   - Cases: usually "v1" — opinions don't change after
    ///     publication; corrections-of-record are rare.
    ///   - Statutes: "as of YYYY-MM-DD" — captures the text
    ///     effective at a specific point in time.
    ///   - Guidelines: "2019_update", "2024_revision", etc.
    ///   - Papers: "v1", "v2_with_corrigendum", etc.
    pub designation: String,
    pub effective_date: Option<String>,    // YYYY-MM-DD
    pub retrieved_at: String,              // when WE decomposed it
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecomposedSource {
    pub key: SourceKey,                    // content hash
    pub identity: SourceIdentity,
    pub version: SourceVersion,
    pub ir: IRDocument,                    // the typed decomposition
    pub provenance: DecompositionProvenance,
    pub claim_index: Option<ClaimIndex>,   // see §"Claim index" below
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecompositionProvenance {
    pub decomposed_at: String,
    pub decomposed_by: String,             // deployment id / agent id
    pub adapter_name: String,              // ADJ39 adapter that fetched it
    pub source_url: String,
    pub source_byte_count: usize,
    pub ir_pipeline_version: String,       // for cache invalidation
                                            // when the pipeline itself
                                            // changes materially
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceKey(pub [u8; 32]);        // BLAKE3 of the source bytes
                                            // + identity + version
```

## Source identity — what counts as "the same source"

The identity must be stable across users + deployments. Different
citation kinds have different canonical identifiers:

| Citation kind | Canonical identifier |
|---|---|
| `CaseLaw` | `reporter + volume + page + court` (one case, anywhere it's cited) |
| `Statute` | `code + section` (one section; versions distinguished separately) |
| `Article` | `DOI` (preferred) or `PMID` (fallback) |
| `SecFiling` | `accession_number` (unique per filing) |
| `Vulnerability` | `CWE-N` / `CVE-YYYY-N` / OWASP entry id |
| `Patent` | `patent_number` (issuance) |

Two CitationFacts pointing to the same source produce the same
`SourceIdentity`, regardless of who cited it or what variation in
quotation/abbreviation appears in the citing text.

## Versioning — what counts as "the same version"

A small subset of source types have versions; most don't.

**Permanent (one version forever):**
- Court opinions (corrections-of-record are rare and tracked
  as annotations, not new versions)
- Published scientific papers (corrigenda are annotations)
- SEC filings (filed once; any restatement is a new filing)
- Issued patents
- CVE entries (deprecation is an annotation)

**Amendable (multiple versions over time):**
- Statutes and regulations (amended periodically)
- Clinical practice guidelines (periodic updates: AGS Beers
  every 4–5 years; HEART score iterations; OWASP Top 10 every
  3-4 years)
- Standards documents (ANSI, IEEE, RFC revisions)

For amendable sources, each version is a separate
`DecomposedSource` keyed by `(identity, version)`. A rulebook
elicited in 2025 citing "OWASP Top 10 2021 A03: Injection" is a
specific-version citation; a rulebook citing the *current* OWASP
Top 10 needs explicit temporal disambiguation (ADJ41 surfaces
this via the `version` field; ADJ-temporal — Gap 3 from ADJ38 —
will handle the resolution semantics).

## Claim index — making lookups fast

Once a source is decomposed, the framework can pre-extract its
holdings / claims into a structured index for fast claim-match
queries. This makes the per-citation cost essentially O(1):

```rust
pub struct ClaimIndex {
    /// One entry per Rule-Fact (holding) extracted from the
    /// source's IR. Sorted by term for fast typed-claim lookup.
    pub claims: Vec<IndexedClaim>,

    /// Embedding vectors for each claim, for semantic similarity
    /// lookup when an exact term match doesn't apply.
    pub embeddings: Option<Vec<Embedding>>,
}

pub struct IndexedClaim {
    pub source_node_id: NodeId,            // the IR node within the source's IR
    pub claim_term: Term,                  // structured
    pub claim_text: String,                // human-readable extract
    pub source_byte_span: (usize, usize),  // where in the source text
    pub claim_kind: ClaimKind,             // holding / dicta / methods /
                                            // results / etc.
    pub weight: f32,                       // domain-specific weighting
                                            // (e.g., holdings > dicta)
}
```

A claim-match query becomes:
1. Look up the source-IR by `SourceIdentity` + `SourceVersion` → O(1)
2. Search the claim_index for terms structurally similar to the
   citing claim → O(log n) or O(1) with embedding index
3. Run the NLI primitive only on the top-k candidate matches → O(k)
   small LLM calls

This is vastly cheaper than re-running the full IR pipeline.

## Federation and sharing

A single deployment's source-IR store grows incrementally as it
encounters new citations. Across deployments, the same set of
canonical authoritative sources gets re-decomposed redundantly —
every law firm decomposes the same federal opinions, every
clinical deployment decomposes the same EBM papers. **Federation
eliminates this waste.**

### Tiered store model

```
                       ┌─────────────────────────┐
                       │  Public source store    │
                       │  (community-maintained, │
                       │   content-addressed,    │
                       │   read-only for users)  │
                       └────────────┬────────────┘
                                    │
                                    │  (mirror / replicate)
                                    │
                       ┌────────────▼────────────┐
                       │  Org / firm store       │
                       │  (firm-private, all     │
                       │   their decompositions, │
                       │   inherits from public) │
                       └────────────┬────────────┘
                                    │
                                    │  (cache subset)
                                    │
                       ┌────────────▼────────────┐
                       │  Local deployment store │
                       │  (per-user; fast cache  │
                       │   of recently-used)     │
                       └─────────────────────────┘
```

### Lookup chain

For each citation:
1. Query local store. If hit → done.
2. Query org store. If hit → cache to local → done.
3. Query public store. If hit → cache to org + local → done.
4. Run IR pipeline on the source. Write back to local + org +
   (with permission) public.

### Public-store maintainability

The public store is a candidate **public-good infrastructure**:
- Anyone can submit a new source decomposition.
- The store deduplicates by content hash (same source → same
  IR regardless of who submitted).
- Submissions are signed (decomposer identity + IR pipeline
  version), so disagreements can be surfaced and resolved.
- A governance layer (analogous to Caselaw Access Project,
  PubMed Central, or Wikipedia) handles disputes.

### Trust model

Public-store entries carry their provenance:

```rust
pub struct DecompositionProvenance {
    pub decomposed_at: String,             // when
    pub decomposed_by: String,             // who (signed)
    pub adapter_name: String,
    pub source_url: String,
    pub source_byte_count: usize,
    pub ir_pipeline_version: String,
}
```

A consuming deployment can require:
- Decompositions only from trusted IDs
- Decompositions only from specific IR pipeline versions
- Cross-validation: re-decompose locally and compare with
  the public entry (rejecting mismatches)

The public-store-as-public-good vision is not mandatory; the
framework's local-store mode is sufficient for single-deployment
use. But the federation option unlocks the asymptotic
zero-marginal-cost claim across the broader user base.

## The cost model, restated

| Phase | Cost |
|---|---|
| Cold start (first citation of a never-decomposed source) | Full IR pipeline on source text + storage write |
| Steady state, local store hit | O(1) lookup + 1 NLI claim-match call |
| Steady state, public store hit | network round-trip + 1 NLI call |
| Asymptotic (mature store, popular source) | dominated by the NLI call (~ms) |

For a deployment processing a corpus of legal briefs, the first
1000 briefs might trigger ~100-500 cold-start decompositions
(many citations repeat). The next 10,000 briefs trigger ~0 cold-
starts — every citation hits the local store. **The store grows
sublinearly with workload because real-world citations follow a
power law: a small set of foundational sources is cited
constantly.**

## Worked example — Mata v. Avianca with a populated store

In the steady-state of a 10,000-brief-processed legal-domain
deployment:

```
Brief filing: New brief cites
  - Hadley v. Baxendale, 9 Exch. 341 (1854)  [contract damages]
  - Twombly, 550 U.S. 544 (2007)              [pleading standard]
  - Iqbal, 556 U.S. 662 (2009)                [pleading standard]
  - Varghese v. China Southern, 925 F.3d 1339 (2d Cir. 2019)
    [fabricated]

Citation 1: Hadley v. Baxendale
  → store lookup: HIT (cited in ~30% of contract briefs in corpus)
  → claim-match against cited proposition: 1 NLI call, ~150ms
  → Verified

Citation 2: Twombly
  → store lookup: HIT (cited in ~80% of federal civil briefs)
  → claim-match: 1 NLI call, ~150ms
  → Verified

Citation 3: Iqbal
  → store lookup: HIT (frequent companion to Twombly)
  → claim-match: 1 NLI call, ~150ms
  → Verified

Citation 4: Varghese
  → store lookup: MISS (never encountered)
  → ADJ39 existence check: CourtListener returns 0 results
  → VerificationStatus::NotFound
  → No IR decomposition performed (source doesn't exist)
  → orchestrator KickBack: refuses to commit

Total verification time: 4 × ~150ms NLI + 1 × ~200ms CourtListener
  ≈ 800ms total
```

The four-citation verification — including catching one fabricated
citation — completes in under a second. **Asymptotic zero-marginal-
cost verification.**

## Update to ADJ40

ADJ40 (#4861) should be updated to reflect this reframe. Concrete
changes:

- Replace §"Cost considerations" with §"Cost model (cold-start
  vs. steady-state)" — referencing this spec
- Replace §"Recursion termination" with §"Source-IR
  decomposition vs. claim-match lookup" — the recursion is
  per-source, not per-query
- Add §"Source store integration" pointing to ADJ41
- Update §"Implementation outline" to use the `SourceIrStore`
  trait

This is a substantive reframe but not a complete rewrite — the
match-strength logic, the depth-1 cap on recursion within an
individual source, and the integration with ADJ39 stand. What
changes is the cost story and the architectural positioning of
the store.

## Implementation roadmap

**Phase 1 — Foundational:**
1. `adjudication-source-store-core` crate: `SourceIrStore` trait
   + `DecomposedSource` types + content-addressing utilities.
2. `adjudication-source-store-fs` implementation: local
   filesystem-backed store with content-addressed entries.
3. `adjudication-source-store-sqlite` implementation: queryable
   indexed store for production single-machine use.

**Phase 2 — Federation:**
4. `adjudication-source-store-http` implementation: read/write
   HTTP API for org + public stores.
5. Reference public-store server implementation (could be
   hosted at e.g. sources.adjudication.org).
6. Signing + verification layer for public-store provenance.

**Phase 3 — Operational:**
7. Bulk-decomposition tool: take a corpus of authoritative
   sources (e.g., Caselaw Access Project bulk data) and
   pre-populate the store.
8. Store-health metrics: cache hit rate, decomposition latency,
   store size growth.

## Status

Draft. Specifies the architectural object that ADJ39 + ADJ40
implicitly depend on; ADJ40 should be updated to reference it.
The implementation roadmap is straightforward — content-
addressed store + IR pipeline integration is well-trodden
engineering; the novel piece is the public-store federation,
which depends on community / governance decisions rather than
technical ones.

## See also

- [ADJ40](ADJ40-recursive-source-decomposition.md) — the spec
  this corrects the cost framing for.
- [ADJ39](ADJ39-citation-verification-infrastructure.md) — the
  spec whose `fetch_full_text` calls populate this store.
- [ADJ38](ADJ38-cross-domain-framework-validation.md) — the
  cross-domain validation that identified this as critical
  infrastructure across all knowledge-work domains.
- [ADJ-temporal](ADJ41-temporal.md) (TBD; Gap 3) — handles the
  resolution semantics for "current version" of amendable
  sources cited without explicit version designation.
