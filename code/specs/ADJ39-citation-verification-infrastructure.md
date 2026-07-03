# ADJ39 — Citation Verification Infrastructure

> The single piece that converts the framework from "good idea
> with hand-waved provenance" to "real system with mechanically-
> verified citations." Specifies the IR variant, the verification
> protocol, the per-domain adapter interface, the verification
> states, the orchestrator integration, the audit-trail recording,
> and the caching / cost-control strategy.
>
> **The framework's central guarantee** — that no inference is
> committed when its supporting citation is unverified — depends
> entirely on this layer. Without ADJ39, ADJ38's Mata v. Avianca
> case is "we *would have* caught it"; with ADJ39, it's "we *did*
> catch it."

## Scope

ADJ39 specifies:

1. **`CitationFact` as a first-class IR variant** (extension to
   ADJ01) — a citation is a Fact with a structured payload
   distinct from a domain claim.
2. **The `VerificationProtocol`** — what every citation-source
   adapter must implement, what the framework expects to receive,
   and what states a verification can return.
3. **Per-domain adapter interfaces** — concrete adapters for
   CourtListener (law), PubMed (medicine), EDGAR (finance),
   CWE/CVE (security), USPTO (patents), state legislative DBs
   (law). Each adapter is a separate crate under
   `adjudication-verify-*`.
4. **Orchestrator integration** — when verification fails,
   what the orchestrator does. (Spoiler: refuses to commit any
   inference dependent on the failed citation; surfaces a
   structured kickback.)
5. **Audit-trail recording** — every verification call produces
   a `VerificationRecord` in the audit trail with the citation,
   the adapter queried, the response, the timestamp, and the
   verifying authority URL.
6. **Caching and cost control** — verification can be expensive
   (rate-limited APIs, paid sources). A content-addressed cache
   makes re-runs free; cost budgets can cap per-document.
7. **Trust tiers** — what to do with sources of varying
   authoritativeness, and how to weight the framework's
   confidence in their answers.

## Why a separate spec

The framework has, until now, treated citations as untyped
metadata. ADJ36 demonstrated end-to-end inference with
hand-written citations; ADJ37 elicited a rulebook with
confidence-marked citations; ADJ38 walked through 7 domains
showing each needs a domain-specific verification adapter. None
of these implemented actual verification — they all hand-waved
("the framework would query PubMed here, but we don't in this
demo").

ADJ39 is the spec that turns those promises into a concrete
interface and implementation contract. **No framework PR after
this one should hand-wave citation verification; the protocol
exists, and either an adapter is wired in or the citation is
flagged as `unverifiable_in_this_environment`.**

## Layer position

```
        ADJ01      ADJ02      ADJ34         ADJ37
        IR         coverage   fallback      rulebook elicitation
        │            │           │              │
        └────────────┴───────────┴──────────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │  Citation-Fact    │   ◀── IR variant added here
                    │  in input or      │
                    │  rulebook IR      │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │  ADJ39            │   ◀── this spec
                    │  Verification     │
                    │  layer            │
                    └─────────┬─────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
          per-domain      per-domain       ...
          adapters        adapters
        (CourtListener,  (PubMed,           (CWE, EDGAR,
         Caselaw)         Crossref)          USPTO, etc.)
                              │
                              ▼
                    ┌───────────────────┐
                    │  Orchestrator     │
                    │  enforces         │
                    │  "no commit       │
                    │   on unverified"  │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │  ADJ07 audit trail │
                    │  records every     │
                    │  verification call │
                    └────────────────────┘
```

## CitationFact — the IR variant

`Fact` in ADJ01 has subtypes implicit in its `term`. ADJ39 adds
`CitationFact` as a first-class variant with structured fields.

```rust
/// A citation as it appears in input or rulebook IR. Distinct
/// from a domain Fact because the framework's verification
/// pipeline operates on the structured fields, not the prose
/// text alone.
#[derive(Debug, Clone, PartialEq)]
pub struct CitationFact {
    pub id: NodeId,
    pub citation_text: String,         // original text as written
    pub source_spans: Vec<Span>,       // where in source these bytes live
    pub citation_kind: CitationKind,
    pub claimed_proposition: Option<String>,  // what the citing
                                               // text claims this
                                               // source supports
    pub provenance: Provenance,
    pub verification: VerificationStatus,  // populated by ADJ39
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CitationKind {
    /// Court opinion / case law
    CaseLaw {
        parties: String,
        reporter: String,
        volume: u32,
        page: u32,
        court: String,
        year: u32,
    },
    /// Statutory or regulatory authority
    Statute {
        code: String,            // e.g., "NY CPLR", "26 U.S.C."
        section: String,         // e.g., "214(5)", "501(c)(3)"
        version_year: Option<u32>,
    },
    /// Scientific/medical journal article (DOI-resolvable)
    Article {
        doi: Option<String>,
        pmid: Option<u64>,        // PubMed identifier
        title: String,
        authors: Vec<String>,
        journal: String,
        year: u32,
        volume: Option<u32>,
        pages: Option<String>,
    },
    /// SEC filing
    SecFiling {
        company_cik: Option<u64>,
        form_type: String,        // "10-K", "S-1", "8-K", ...
        period_end: Option<String>,  // "2024-12-31"
        accession_number: Option<String>,
    },
    /// CWE / CVE / OWASP vulnerability database entry
    Vulnerability {
        identifier: String,       // "CWE-89", "CVE-2024-12345"
        database: String,         // "CWE", "CVE", "OWASP"
        version: Option<String>,
    },
    /// USPTO patent or trademark
    Patent {
        patent_number: String,
        kind_code: Option<String>, // "A1", "B2", ...
        publication_date: Option<String>,
    },
    /// Trademark
    Trademark {
        registration_number: String,
        country: String,
    },
    /// Press / news source
    Press {
        publisher: String,
        url: Option<String>,
        title: String,
        published_date: Option<String>,
    },
    /// Other — needs human review to determine source_kind.
    Other {
        description: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Provenance {
    /// Extracted from input source (citation cited in the
    /// document being adjudicated)
    FromInput,
    /// Elicited by LLM as part of rulebook generation
    FromLLMElicitation {
        elicitation_prompt_hash: String,
        elicitation_confidence: ElicitationConfidence,
    },
    /// Provided by trusted human-curated rulebook
    FromTrustedRulebook {
        rulebook_id: String,
        rulebook_version: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElicitationConfidence {
    High,     // LLM was confident on paper + claim
    Medium,   // paper confident, claim approximate
    Low,      // both uncertain
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationStatus {
    /// Not yet verified; orchestrator will run verification
    /// before any inference depends on this Fact.
    Pending,
    /// Verified to exist and support the claimed proposition.
    Verified {
        verified_at: String,
        adapter_name: String,
        authoritative_url: String,
        match_score: f32,         // how cleanly did the proposition match
    },
    /// Citation does not exist in the queried authority.
    NotFound {
        verified_at: String,
        adapter_name: String,
        detail: String,
    },
    /// Citation exists but doesn't support the claimed
    /// proposition (e.g., wrong holding, wrong statute scope).
    WrongClaim {
        verified_at: String,
        adapter_name: String,
        actual_content_summary: String,
        detail: String,
    },
    /// Verification was attempted but the source is currently
    /// unreachable (rate limit, network error, paid-source
    /// access denied). Treated as unverified; framework refuses
    /// to commit until verification completes.
    Unreachable {
        attempted_at: String,
        adapter_name: String,
        reason: String,
    },
    /// No adapter is configured for this CitationKind.
    /// Framework cannot verify; treated as unverified.
    NoAdapter {
        citation_kind: String,
    },
    /// Citation exists, but was superseded / overruled /
    /// retracted. Framework records but flags the inferential
    /// weight as reduced.
    Superseded {
        verified_at: String,
        adapter_name: String,
        replaced_by: Option<String>,
        treatment_history: String,
    },
}
```

## The VerificationProtocol

Every citation-source adapter implements one trait:

```rust
#[async_trait]
pub trait CitationVerifier: Send + Sync {
    /// Name of this adapter (e.g., "courtlistener-v1", "pubmed-v3").
    fn name(&self) -> &str;

    /// Which CitationKind this adapter handles.
    fn handles(&self) -> CitationKind;  // pattern match: matches the
                                        // shape, ignores instance-specific
                                        // fields

    /// Verify the citation. Returns VerificationStatus.
    ///
    /// Adapters MUST:
    ///   - Be deterministic for a given (citation, claimed_proposition)
    ///     pair: same query → same answer at the same point in time.
    ///   - Return Unreachable rather than panicking on transient failures.
    ///   - Return NoAdapter if the citation_kind shape is not handled.
    ///   - Cache responses internally (or via the framework's cache layer).
    async fn verify(
        &self,
        citation: &CitationFact,
    ) -> VerificationStatus;

    /// (Optional) Fetch the full text of the cited source, for
    /// recursive IR processing per ADJ40. Returns None if the
    /// adapter cannot retrieve full text.
    async fn fetch_full_text(
        &self,
        citation: &CitationFact,
    ) -> Option<String>;

    /// Cost estimate for verifying one citation, in arbitrary
    /// abstract units (the framework's cost budget normalizes
    /// across adapters).
    fn cost_estimate(&self) -> CostEstimate;
}

pub struct CostEstimate {
    pub api_units: u32,      // arbitrary; CourtListener=1, Westlaw=10
    pub estimated_ms: u32,   // latency
    pub free: bool,          // does this hit a paid source
}
```

## Per-domain adapters

### Adapter 1: CourtListener (legal case law, FREE)

CourtListener is the Free Law Project's open-access caselaw
database. Has REST + bulk API. Covers federal + many state
appellate courts back to ~1850.

```rust
pub struct CourtListenerVerifier {
    api_token: Option<String>,  // optional; works without for low-volume
    cache: Arc<dyn VerifyCache>,
}

impl CitationVerifier for CourtListenerVerifier {
    fn name(&self) -> &str { "courtlistener-v1" }

    fn handles(&self) -> CitationKind {
        CitationKind::CaseLaw { ..Default::default() }
    }

    async fn verify(
        &self,
        citation: &CitationFact,
    ) -> VerificationStatus {
        let CitationKind::CaseLaw { reporter, volume, page, court, year, parties } = &citation.citation_kind
        else { return VerificationStatus::NoAdapter { ... }; };

        // Cache check
        let key = format!("courtlistener:{}:{}:{}:{}", reporter, volume, page, year);
        if let Some(cached) = self.cache.get(&key) {
            return cached;
        }

        // Query: GET https://www.courtlistener.com/api/rest/v4/search/
        //        ?q="<parties>"&type=o&reporter=<reporter>&volume=<volume>&page=<page>
        let result = self.api_query(parties, reporter, *volume, *page, court, *year).await;

        let status = match result {
            Err(transient) => VerificationStatus::Unreachable { ... },
            Ok(None) => VerificationStatus::NotFound {
                verified_at: now(),
                adapter_name: self.name().to_string(),
                detail: format!(
                    "No case matches {reporter} {volume} at {page} in {court} {year}; \
                     CourtListener returned 0 results."),
            },
            Ok(Some(opinion)) => {
                // If a claimed_proposition is set, fetch the opinion text
                // and run a recursive IR check (ADJ40); for ADJ39 v0.1
                // we just verify existence.
                VerificationStatus::Verified {
                    verified_at: now(),
                    adapter_name: self.name().to_string(),
                    authoritative_url: opinion.absolute_url,
                    match_score: 1.0,
                }
            }
        };

        self.cache.put(&key, status.clone());
        status
    }

    async fn fetch_full_text(&self, citation: &CitationFact) -> Option<String> {
        // Implementation: query CourtListener for the opinion HTML/PDF,
        // strip to plain text, return.
        ...
    }

    fn cost_estimate(&self) -> CostEstimate {
        CostEstimate { api_units: 1, estimated_ms: 200, free: true }
    }
}
```

**Coverage estimate**: CourtListener has the vast majority of
federal opinions and most state appellate opinions back several
decades. **The Mata v. Avianca case would have been caught by
this adapter alone** — Varghese v. China Southern Airlines does
not exist in CourtListener; the query returns 0 results.

### Adapter 2: PubMed (medical / scientific literature, FREE)

NCBI's E-utilities API. Free, no auth required (or use API key
for higher rate limits).

```rust
pub struct PubMedVerifier {
    api_key: Option<String>,
    cache: Arc<dyn VerifyCache>,
}

impl CitationVerifier for PubMedVerifier {
    fn name(&self) -> &str { "pubmed-v3" }

    fn handles(&self) -> CitationKind {
        CitationKind::Article { ..Default::default() }
    }

    async fn verify(&self, citation: &CitationFact) -> VerificationStatus {
        let CitationKind::Article { doi, pmid, title, authors, journal, year, .. }
            = &citation.citation_kind else { return ...; };

        // Try PMID first if provided, else DOI, else search by title + first
        // author + year.
        if let Some(pmid) = pmid {
            // GET https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?
            //     db=pubmed&id={pmid}&retmode=json
            ...
        } else if let Some(doi) = doi {
            // Resolve DOI via Crossref or PubMed's idconv tool
            ...
        } else {
            // Title + author search
            ...
        }
        // ...
    }

    fn cost_estimate(&self) -> CostEstimate {
        CostEstimate { api_units: 1, estimated_ms: 250, free: true }
    }
}
```

### Adapter 3: SEC EDGAR (financial filings, FREE)

EDGAR full-text search + filing retrieval. Free, no auth.

```rust
pub struct EdgarVerifier {
    cache: Arc<dyn VerifyCache>,
}

impl CitationVerifier for EdgarVerifier {
    fn name(&self) -> &str { "edgar-v1" }

    fn handles(&self) -> CitationKind {
        CitationKind::SecFiling { ..Default::default() }
    }

    async fn verify(&self, citation: &CitationFact) -> VerificationStatus {
        let CitationKind::SecFiling { company_cik, form_type, period_end, accession_number }
            = &citation.citation_kind else { return ...; };

        // If accession_number is provided, look it up directly:
        // GET https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={cik}
        //     &type={form}&dateb=&owner=include&count=10&action=getcompany
        ...
    }

    fn cost_estimate(&self) -> CostEstimate {
        CostEstimate { api_units: 1, estimated_ms: 300, free: true }
    }
}
```

### Adapter 4: CWE/CVE (security vulnerabilities, FREE)

CWE published as XML by MITRE; CVE accessible via NVD API.

```rust
pub struct CweVerifier { ... }
pub struct CveVerifier { ... }
pub struct OwaspVerifier { ... }
```

### Adapter 5: USPTO (patents, FREE)

PatentsView API for patent lookups.

```rust
pub struct PatentsViewVerifier { ... }
```

### Adapter 6: State legislative DBs (variable, mostly FREE)

Per-state adapters. Some states have open APIs; others require
scraping. Initial coverage: California (open data), New York
(legislation.nysenate.gov), Texas (capitol.texas.gov),
Florida (flsenate.gov). Federal-only is fully covered by
Library of Congress Congress.gov API.

### Adapter 7: Westlaw / Lexis (paid, OPTIONAL)

Behind paywalls. The framework supports configuration to call
these for premium users; runs without them otherwise. The
framework's free-tier guarantee depends only on the
free-adapter set above.

## Orchestrator integration

When the orchestrator's coverage pass completes, it has an IR
containing some `CitationFact` nodes (in input IR and/or
rulebook IR). Before allowing any inference dependent on those
citations to commit, the orchestrator runs:

```python
def verify_all_citations(ir):
    failures = []
    for fact in ir.nodes:
        if isinstance(fact, CitationFact):
            adapter = find_adapter_for(fact.citation_kind)
            if adapter is None:
                fact.verification = VerificationStatus.NoAdapter(...)
                failures.append(fact)
                continue
            status = adapter.verify(fact)
            fact.verification = status
            if status is not Verified and status is not Superseded:
                failures.append(fact)
    return failures
```

The orchestrator's decision:

```python
if failures:
    return KickBack(
        question=structured_question_from_failures(failures),
        unverified_citations=failures,
        # do NOT commit to any inference depending on these
    )
else:
    # All citations verified; proceed to engine inference
    return run_engine(ir)
```

Inference dependent on `Superseded` citations is *allowed* but
flagged in the audit trail with the supersession history.

## Audit-trail recording

Every verification call produces a `VerificationRecord` in the
audit trail:

```rust
pub struct VerificationRecord {
    pub citation_node_id: NodeId,        // which CitationFact
    pub adapter_name: String,            // which adapter ran
    pub attempted_at: String,            // ISO-8601 timestamp
    pub completed_at: String,
    pub status: VerificationStatus,
    pub authoritative_url: Option<String>,
    pub cache_hit: bool,
    pub cost_units_consumed: u32,
}
```

These records are part of the `AdjudicationOutcome` and serialize
into the ADJ07 audit-trail JSON. A reviewer can replay every
verification by re-running the same adapter with the same
citation; the framework's cache makes this fast.

## Caching and cost control

Verification can be expensive (rate-limited APIs, paid sources).
Three layers:

1. **In-memory LRU cache** per process — same-process re-runs
   are O(1) (used during a single adjudication that touches the
   same citation multiple times across rulebook and input).
2. **On-disk content-addressed cache** — keyed by adapter name +
   citation structured fields + adapter version. Survives
   process restarts; lets a re-run of any past adjudication
   replay verifications without re-hitting the source.
3. **Audit-trail-as-cache** — past `VerificationRecord`s in any
   completed adjudication audit can be queried by hash. The
   framework treats stored audit trails as authoritative for
   immutable sources (a 1995 published case law citation doesn't
   change).

**Cost budgets**: the framework configuration can specify
`max_cost_per_adjudication: u32` (e.g., 100 units). If
verification exceeds this, the framework returns a partial
verification result and a kickback noting cost-budget exhaustion.

## Trust tiers

Not all sources are equally authoritative:

| Tier | Examples                                | Treatment |
|------|----------------------------------------|-----------|
| 1    | Official court reporters (F.3d, U.S.)   | Authoritative; verified content used as ground truth |
| 1    | Government-published statutes/regs      | Authoritative |
| 1    | PubMed / DOI-resolved articles          | Authoritative |
| 1    | SEC EDGAR filings (SEC.gov)             | Authoritative |
| 2    | CourtListener-imported state cases      | Authoritative if matches Tier 1 (US Caselaw Project), else high-confidence |
| 2    | CWE / CVE / OWASP                       | Authoritative for their domain |
| 3    | Peer-reviewed journal aggregators       | High-confidence |
| 4    | Press releases (companies, agencies)    | Acknowledged but flagged; not authoritative for primary claims |
| 4    | Wikipedia                               | Useful for orientation; never authoritative for a citable claim |
| 5    | Blog posts, Medium, social media        | Not citable for primary claims |

The Trust tier is recorded in the `VerificationRecord`. The
audit-trail consumer (downstream consumer of framework output)
can apply policy: e.g., "all primary claims must cite Tier 1 or 2."

## Failure-mode coverage matrix

For each citation kind, what failure modes is the framework
guaranteed to catch?

| Citation kind | Existence | Wrong content | Wrong jurisdiction | Stale/Superseded | Wrong scope |
|---|---|---|---|---|---|
| CaseLaw | ✓ (CourtListener) | ✓ (with ADJ40 recursion) | ✓ (court field) | ✓ (citator) | ✓ (with ADJ40) |
| Statute | ✓ (Legislative DBs) | ✓ (with ADJ40 recursion) | ✓ (code field) | ✓ (version_year) | ✓ (with ADJ40) |
| Article | ✓ (PubMed/Crossref) | ✓ (with ADJ40 recursion) | n/a | ✓ (retractions DB) | ✓ (with ADJ40) |
| SecFiling | ✓ (EDGAR) | ✓ (with ADJ40 recursion) | n/a | n/a | ✓ |
| Vulnerability | ✓ (CWE/CVE/OWASP) | ✓ (database content) | n/a | ✓ (deprecated entries) | ✓ |
| Patent | ✓ (USPTO) | ✓ (with ADJ40 recursion) | n/a | ✓ (status field) | ✓ |
| Press | ✓ (URL probe) | ⚠️ (text retrieval) | n/a | n/a | n/a |

ADJ40 (recursive source decomposition) is the dependency for
"wrong content" checks — verifying that the cited source
actually supports the claimed proposition requires processing
the source's full text. ADJ39 v0.1 verifies *existence*; ADJ40
adds *content match*. Both are needed for the Mata v. Avianca
guarantee in full generality.

## Mata v. Avianca, replayed

The companion executor (`adj38-mata-avianca-demo.py`) mocked
verification because we have no network in that demo. The full
ADJ39 implementation would replay as follows:

```
Input: brief paragraph (152 bytes citing Varghese)

IR extraction (ADJ01–02): CitationFact(
    citation_text="Varghese v. China Southern Airlines, 925 F.3d 1339 (2d Cir. 2019)",
    citation_kind=CaseLaw {
        parties: "Varghese v. China Southern Airlines",
        reporter: "F.3d",
        volume: 925,
        page: 1339,
        court: "2d Cir.",
        year: 2019,
    },
    claimed_proposition: "statute-of-limitations defenses are non-waivable in diversity actions",
    provenance: FromInput,
    verification: Pending,
)

ADJ39 protocol invocation:
    adapter = CourtListenerVerifier::new()
    status = adapter.verify(citation).await

    → HTTP GET https://www.courtlistener.com/api/rest/v4/search/
                ?q=%22Varghese+v.+China+Southern+Airlines%22
                &type=o&reporter=F.3d&volume=925

    → Response: { "count": 0, "results": [] }

    → VerificationStatus::NotFound {
        verified_at: "2026-06-02T03:24:00Z",
        adapter_name: "courtlistener-v1",
        detail: "Zero hits for parties + reporter + volume.",
      }

Orchestrator's decision:
    failures = [citation_id]
    return KickBack(
        question="The cited case Varghese v. China Southern
                  Airlines, 925 F.3d 1339 (2d Cir. 2019) does
                  not exist in CourtListener (free, comprehensive
                  open-access database covering 2d Cir. opinions).
                  This appears to be a fabricated citation.
                  Action required: replace with a verifiable
                  authority, or remove the proposition.",
        unverified_citations=[citation],
    )

Audit trail records:
    - The original brief text (input bytes 0..152)
    - The extracted CitationFact (NodeId C1)
    - The VerificationRecord for C1 (CourtListener queried,
      0 results, status NotFound)
    - The orchestrator's KickBack decision (refused to commit)
```

**The framework refuses to commit. The Mata v. Avianca scenario
is structurally prevented.** This is the guarantee ADJ39 makes
real.

## What ADJ39 v0.1 ships (and what's deferred)

**v0.1 (this spec):**
- The CitationFact IR variant
- The CitationVerifier trait
- Adapter implementations for CourtListener, PubMed, EDGAR, CWE
  (the four highest-value free APIs)
- Verification states + audit-trail recording
- LRU + on-disk caching
- Trust tier classification

**Deferred to ADJ40:**
- Full-text retrieval and recursive IR processing of cited
  sources
- "Wrong content" verification (verifying the cited paper
  actually contains the claimed claim)

**Deferred to follow-up ADJ-versioning spec:**
- Adapter for paid sources (Westlaw, Lexis) with consent /
  configuration
- State legislative DB adapters (per-state, ongoing work)
- USPTO patent claim-content verification

## Implementation roadmap

Phase 1 — Foundational:
1. New crate `adjudication-verify-core` defining the trait + IR
   variant + cache.
2. New crate `adjudication-verify-courtlistener`.
3. Pipeline integration: orchestrator runs verification before
   commit.

Phase 2 — Coverage:
4. `adjudication-verify-pubmed`.
5. `adjudication-verify-edgar`.
6. `adjudication-verify-cwe-cve`.

Phase 3 — Operational:
7. CLI tool that takes a corpus of cited briefs / papers /
   filings and reports verification status across all.
8. Web UI for reviewing verification reports.

## Status

Draft spec. The interface, protocol, and adapter shapes are
specified. Implementation is the next-natural sequence of PRs.

## See also

- [ADJ38](ADJ38-cross-domain-framework-validation.md) — the
  cross-domain validation that motivated this spec; ADJ39 is
  Gap 1 from that document.
- [ADJ37](ADJ37-unified-framework-and-rulebook-elicitation-demo.md)
  — the unified-framework demo that surfaced the need for
  verification; the rulebook elicitation flagged citations as
  HIGH/MEDIUM/LOW confidence without any actual lookup. ADJ39
  makes the lookups real.
- [ADJ07](ADJ07-audit-trail-schema.md) — the audit trail this
  spec extends with VerificationRecord.
- [ADJ40](ADJ40-recursive-source-decomposition.md) — the next-
  natural spec that adds content-match verification on top of
  ADJ39's existence verification.
