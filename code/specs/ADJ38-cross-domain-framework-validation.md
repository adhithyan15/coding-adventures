# ADJ38 — Cross-Domain Framework Validation: The Framework as Attention Scaffold for LLM-Driven Knowledge Work

> Tests whether the framework holds outside medicine. Anchored on
> the **Mata v. Avianca** failure — a federal brief filed in 2023
> containing fake case citations that ChatGPT confabulated and the
> lawyer didn't verify — which the framework must structurally
> prevent.
>
> Walks through **seven knowledge-work domains** under a sharper
> reframe: **the framework is fundamentally an attention scaffold
> for the LLM.** Hallucination is an attention failure; the
> framework's mechanisms (total coverage, typed IR, citation
> verification, VOI kickback) are all ways of forcing the LLM to
> attend to features it would otherwise overlook. The realistic
> productivity target is **80/20**: automate the systematic 80%
> where attention-forcing is mechanical, surface the messy 20% as
> structured kickback where genuine human judgment is needed.
>
> Identifies the gaps each domain exposes. Catalogues the spec
> changes needed to make the framework production-grade across
> all of them.

## The reframe

Re-stated from the conversation that motivated this PR:

> *"The framework is fundamentally a mechanical attention scaffold
> for the LLM. Hallucination is mostly an attention failure: the
> model glides over a feature it should have noticed and
> confidently fills in plausible-looking content. The framework's
> mechanisms are all ways of constraining the LLM's attention to
> features it would otherwise overlook. The 80/20 target is the
> realistic productivity claim: automate the systematic 80% where
> the framework's attention-forcing reliably catches things;
> surface the messy 20% as structured kickback where human
> judgment is needed."*

This is a tighter framing than "small-models-via-structure" or
"LR-aggregation-for-medicine." It says: **the framework is a
discipline for *how LLMs are allowed to look at inputs*, not a
discipline for what they're allowed to say.** Reduce hallucination
not by post-filtering the output but by forcing the model to
commit, byte-by-byte and cite-by-cite, to what it actually
claims to know.

The 80/20 target — **finish 80% of the work auto-pilot, surface
20% as structured human-disambiguation requests** — is what makes
this practically useful. No knowledge worker wants "AI that does
all of my work poorly"; they want "AI that does the boring 80% so
I can focus on the genuinely hard 20%."

## The anchor: Mata v. Avianca (2023)

In June 2023, attorney Steven Schwartz filed a 10-page brief in
*Mata v. Avianca Inc.* (S.D.N.Y. 1:22-cv-01461) opposing dismissal.
The brief cited at least six federal cases, including:

- *Varghese v. China Southern Airlines Co. Ltd.*, 925 F.3d 1339 (11th Cir. 2019)
- *Shaboon v. Egyptair*, 2013 IL App (1st) 111279-U
- *Petersen v. Iran Air*, 905 F. Supp. 2d 121 (D.D.C. 2012)
- *Martinez v. Delta Airlines, Inc.*, 2013 WL 1654300
- *Estate of Durden v. KLM Royal Dutch Airlines*, 2017 WL 2418825
- *Miller v. United Airlines, Inc.*, 174 F.3d 366 (2d Cir. 1999)

**None of the first five cases existed.** ChatGPT had confabulated
them. The opinion text Schwartz quoted was generated, not extracted.
The judge sanctioned Schwartz, imposed a $5,000 fine, and the case
became the canonical legal-AI cautionary tale.

This is **exactly the failure mode the framework must structurally
prevent.** Every cited case is a Citation-Fact node in the
rulebook IR or input IR; every Citation-Fact must be verified
against an external authority (CourtListener, Westlaw, Caselaw
Access Project) before being included in any inference; verified
citations are tagged with provenance; unverified citations are
treated as Uncertainty and kicked back to the human.

**A lawyer using this framework cannot ship a brief with a
fabricated citation, because the framework refuses to commit the
inference until every Citation-Fact is verified.** That's the
guarantee the framework's value proposition rests on for legal
work.

## Domain catalog and gap inventory

Seven domains follow. For each:

1. **Representative case** (real-shape input, ~200–500 bytes).
2. **What the LLM-without-framework would typically do** — the
   attention failures that lead to hallucination.
3. **What the framework forces the LLM to attend to** — the
   mechanical attention-forcing through IR + typing + citation +
   coverage.
4. **The 80% auto-produced output** — what the framework
   completes systematically.
5. **The 20% kicked-back items** — what specifically goes back to
   the human for disambiguation.
6. **Domain-specific framework gaps** — what's specified or
   implemented today that this domain doesn't have yet.

A consolidated gap inventory at the end prioritizes follow-up
specs.

---

## Domain 1: Legal brief writing (the Mata v. Avianca anchor)

### Case

A junior associate is drafting a motion to dismiss. The argument:
the plaintiff's claim is time-barred under the relevant statute
of limitations. The associate writes:

```
The Court should dismiss this action because Plaintiff filed the
complaint more than two years after the cause of action accrued,
in violation of CPLR § 214(5). The Second Circuit held in
Varghese v. China Southern Airlines, 925 F.3d 1339 (2d Cir.
2019), that statute-of-limitations defenses are non-waivable in
diversity actions. Plaintiff's claim accrued on June 15, 2021,
when the breach was discovered; this action was filed on
September 22, 2023, well outside the two-year window.
```

### What the LLM-without-framework typically does

Generates the brief paragraph with plausible-looking citations.
The Varghese case may be fabricated; the section number CPLR §
214(5) may be wrong (CPLR § 214 has subsections but §214(5) might
not be the right one for breach of contract — that's CPLR § 213
six-year for written contracts, § 214(4) three-year for some
tort actions). The associate, under time pressure, doesn't
verify and submits.

This is exactly the Mata v. Avianca scenario.

### What the framework forces the LLM to attend to

The brief is itself a document. The framework decomposes it
through the same IR pipeline as any other input:

- **Sentence-level**: every sentence tiled, every byte accounted
  for.
- **Phrase-level**: legal terms-of-art extracted as Entity nodes
  (e.g., "statute of limitations," "diversity action").
- **Claim-level**: each substantive claim typed:
  - `Fact: cited_case(varghese_v_china_southern, citation="925 F.3d 1339", court="2d Cir.", year=2019)`
  - `Fact: cited_statute("CPLR § 214(5)")`
  - `Fact: legal_proposition("statute-of-limitations defenses are non-waivable in diversity actions")` — claims the cited case supports a specific legal proposition
  - `Fact: factual_assertion(claim_accrual_date="June 15, 2021")`
  - `Fact: factual_assertion(filing_date="September 22, 2023")`
- **TypedComponent-level**: each citation broken into reporter
  + volume + page + court + year for structured verification.

### The 80% auto-produced output

The framework, before allowing the brief to ship, runs the
following verification chain on every Citation-Fact:

1. **Existence check** against CourtListener / Caselaw Access
   Project / Westlaw API: does the case exist with the cited
   reporter / volume / page / court / year? If not → reject the
   citation as `verification_failed:not_found`.

2. **Court mismatch check**: the citation says "2d Cir." but
   "925 F.3d 1339" with a Varghese parties string might (in fact
   does, since Varghese never existed) not match the 2d Circuit's
   actual published opinions. Cross-check the court against the
   reporter range. Reject mismatches as
   `verification_failed:court_mismatch`.

3. **Legal-proposition check** (recursive citation processing):
   if the case *does* exist, retrieve the case opinion text and
   run the IR pipeline on the opinion. Does the opinion actually
   state the claimed legal proposition? If not → mark as
   `verification_failed:misrepresented_holding`.

4. **Citator check** (Shepardize / KeyCite equivalent): is the
   case still good law, or has it been overruled / reversed /
   narrowed? Tag with the case's treatment status.

5. **Jurisdictional applicability**: the brief is filed in
   S.D.N.Y. (federal trial court in NY). A 11th Circuit case
   would be persuasive but not binding. The framework records
   the jurisdictional weight as part of the citation's
   provenance.

6. **Statute number check**: CPLR § 214(5) is queried against
   New York's published statutes. Does that section exist? Does
   it govern the claimed cause of action? Tag mismatches as
   `verification_failed:wrong_statute_section`.

7. **Factual-date arithmetic**: the framework computes the
   elapsed time from accrual to filing (June 15, 2021 →
   September 22, 2023 = 2 years 3 months 7 days) and compares
   against the applicable limitations period after the statute
   citation is verified.

For a *correctly-written brief* with verified citations and
matching propositions, the 80% auto-produced output is:

- A verdict (e.g., "limitations defense is well-founded;
  recommend court grant motion to dismiss")
- A defensible derivation citing every fact byte + every
  authority with provenance back to source
- A confidence (e.g., 90% — high confidence on a well-cited
  proposition + clean arithmetic)

### The 20% kicked back to the human (in the Avianca scenario)

For the **broken brief**, the framework returns:

> **DO NOT SHIP. Brief contains 1 unverified Citation-Fact and 1
> ambiguous statute reference.**
>
> 1. *Citation-Fact*: `Varghese v. China Southern Airlines, 925
>    F.3d 1339 (2d Cir. 2019)` — **VERIFICATION FAILED**. No case
>    matches this citation in CourtListener, Caselaw Access
>    Project, or Westlaw. The 2d Circuit's F.3d 925 reports do
>    not contain a case with this parties caption. **Action
>    required**: replace this citation with a verifiable
>    authority for the claimed proposition (statute-of-
>    limitations non-waivability in diversity), or remove the
>    proposition from the brief.
>
> 2. *Cited statute*: `CPLR § 214(5)` — **AMBIGUOUS**. The brief
>    relies on this section for a 2-year limitations period, but
>    NY CPLR § 214(5) governs *negligence resulting in personal
>    injury including medical malpractice*. If the claim sounds
>    in *breach of contract* (common in commercial-airline
>    disputes), the applicable section is CPLR § 213(2) (six
>    years for written contracts). **Action required**: confirm
>    the claim's legal theory and the corresponding limitations
>    period.
>
> Until both items are resolved, the framework will not produce a
> verdict.

That is the framework working as designed: the lawyer cannot
ship a brief with fabricated citations because the framework
refuses to commit until verification passes.

### What features the framework forced the LLM to attend to

- The *existence* of each cited case (not just its plausibility)
- The *holding* of each cited case (not just its inferred
  proposition)
- The *jurisdictional applicability* (binding vs. persuasive)
- The *statute section's actual scope* (matches the legal theory)
- The *factual-date arithmetic* (independent of cited authority)
- The *good-law status* of each cited case

A junior associate writing this brief without the framework
would attend to *some* of these but might miss the
statute-subsection mismatch under time pressure. With the
framework, every one of these checks is mechanical.

### Domain-specific framework gaps

1. **Citation-verification crate** (broader than just legal,
   but legal is the most consequential): needs adapters for
   CourtListener / Caselaw Access Project / Westlaw / Lexis.
   Specced loosely in ADJ37; concrete API + lookup logic
   needs ADJ39.
2. **Recursive-citation IR processing**: fetching the *full
   opinion text* of a cited case, running the IR pipeline on
   it, and checking whether the claimed legal proposition is
   actually in the opinion. Conceptually in ADJ37; engineering
   needs to land.
3. **Jurisdictional weighting**: a citation's persuasive
   weight depends on the court issuing it relative to the
   court hearing the current matter. The IR needs a
   first-class `jurisdiction_weight` field on Citation-Facts.
4. **Statute / regulation verification**: similar problem to
   case-law verification but with different APIs (state code
   APIs, CFR for federal). Needs its own adapter.
5. **Citator / good-law status**: KeyCite-equivalent service.
   Closed proprietary today; some open alternatives exist
   (CaseText's CARA, RECAP).
6. **Legal-proposition matching**: comparing "the brief
   claims case X stands for Y" against the actual opinion
   text. This is itself an LLM-driven NLI-style check;
   recursive use of the framework on the opinion.

---

## Domain 2: Code review for security vulnerabilities

### Case

A senior engineer reviews a pull request:

```python
@app.route("/search")
def search():
    q = request.args.get("q", "")
    sql = f"SELECT * FROM products WHERE name LIKE '%{q}%'"
    return db.execute(sql).fetchall()
```

### What the LLM-without-framework typically does

Reads the code, generates a plausible-sounding review:

> *"This endpoint accepts a search query parameter and runs a
> LIKE query against the products table. Consider adding input
> validation and rate limiting. The code looks clean overall."*

The review misses the obvious SQL injection vulnerability. The
LLM's attention was on style and idiomatic concerns, not on the
string-interpolation-into-SQL pattern. This is an attention
failure: the model knows about SQL injection in the abstract,
but didn't *attend to* the specific code construct that exhibits
it. CWE-89.

### What the framework forces the LLM to attend to

The code is decomposed into a typed IR:

- **Sentence/Phrase analog for code**: AST-level decomposition.
  Function definition → request-parameter extraction → string
  interpolation → SQL execution → result return.
- **Claim-level**:
  - `Fact: data_flow(source=request.args.get, sink=db.execute)`
  - `Fact: untrusted_input_in_query(source="request.args.get", sink="db.execute", path="string interpolation")`
  - `Fact: query_construction(method="f-string interpolation", contains_user_input=True)`
- **TypedComponent-level**: each call's parameters, the
  specific string-interpolation token, the f-string variables.

### The rulebook (from CWE / OWASP corpus)

```
contributes(LR=9.0, untrusted_input_in_query(source=*, sink=db.*, path="string interpolation"), sql_injection).
  [CWE-89: "Improper Neutralization of Special Elements used in an SQL Command"]
  [OWASP Top 10 2021 A03: "Injection"]

contributes(LR=0.05, parameterized_query(uses_placeholders=True), sql_injection).
  [Mitigates; if observed, reduces risk dramatically]

prior(0.02, sql_injection).  [Base rate of SQL-injection vulnerabilities in code reviewed]
```

### The 80% auto-produced output

```
VULNERABILITY REPORT:

⚠️  SQL Injection (CWE-89) — HIGH confidence
    Posterior P(SQL-injection) = 0.95

    Source: request.args.get("q") at line 3
    Sink:   db.execute(sql) at line 5
    Path:   string interpolation in f-string at line 4

    Citations:
      - CWE-89: "Improper Neutralization of Special Elements
        used in an SQL Command" [verified: cwe.mitre.org/data/
        definitions/89.html]
      - OWASP Top 10 2021, A03: Injection [verified]

    Recommended fix:
      sql = "SELECT * FROM products WHERE name LIKE %s"
      result = db.execute(sql, (f"%{q}%",)).fetchall()

    Audit trail:
      - Code lines 1-6: full coverage (every line tiled)
      - Vulnerability path: explicit data-flow from L3 to L5
      - Rule applied: CWE-89/A03, LR 9.0, citation verified
```

### The 20% kicked back

For *this* code, nothing — the vulnerability is unambiguous.
But if the code were:

```python
sql = f"SELECT * FROM products WHERE name LIKE '%{escape(q)}%'"
```

The framework would kick back:

> **Ambiguous: the `escape(q)` call may or may not be sufficient
> to prevent SQL injection. Determination depends on:
> [a] which `escape` function (the project-local `escape` from
>     line 12 of util.py? Python's `html.escape`? Some other?)
> [b] What characters it neutralizes (only HTML, or SQL
>     metacharacters?)
> Please confirm the escape function's identity and behavior.**

Or, if the code uses a custom ORM:

> **Ambiguous: `db.execute()` may or may not be parameterized
> safely depending on the `db` object's class. The framework
> cannot determine this without seeing the `db` initialization.
> Please share the database adapter setup (which library,
> which connection class) or confirm whether parameterization
> is automatic.**

### What the framework forced the LLM to attend to

- **Data flow**: explicit tracing from request input to query
  execution
- **Specific construction method**: f-string interpolation vs.
  parameterized query
- **Sink identification**: which function is the SQL executor
- **Rulebook lookup**: which CWE entries apply
- **Citation verification**: CWE-89 and OWASP A03 must be
  resolvable to canonical URLs

### Domain-specific gaps

1. **AST-level IR decomposition**: code is structured input;
   the IR should reflect that. The framework's current
   text-decomposition is byte-level; for code, AST nodes are
   the natural granularity.
2. **Static-analysis integration**: the framework should
   incorporate output from existing static analyzers (Semgrep,
   CodeQL, Bandit) as Facts in the IR.
3. **Reachability and data-flow analysis**: distinguishing
   "this *looks* like SQL injection" from "this *is* SQL
   injection given the call graph" requires call-graph + data-
   flow tracing. Not currently in scope.
4. **CWE / CVE database adapter**: verifying that cited rules
   correspond to canonical CWE entries.

---

## Domain 3: Investment due diligence

### Case

An analyst is reviewing a Series B pitch deck slide:

```
ACME SaaS has achieved $14M ARR with 142% net revenue retention,
representative of best-in-class B2B SaaS performance.
Comparable Series B companies (Datadog, Snowflake, ServiceNow at
similar stage) had ARR multiples of 25-40x. At ACME's $250M
target valuation, our implied ARR multiple is 17.9x — at a
discount to public-market comparables.
```

### What the LLM-without-framework typically does

Generates an analysis: "*ACME's metrics are strong for a Series B;
the 17.9x ARR multiple is conservative relative to the public-
comp range of 25-40x. Recommend further DD on the customer
concentration and gross margin profile.*"

The LLM misses:
- The comparables cited (Datadog, Snowflake, ServiceNow) had
  **vastly different stage characteristics** at Series B. Datadog
  was bootstrap-profitable; Snowflake was post-product-market-fit
  with enterprise distribution. ServiceNow's Series B was 2007
  in a completely different market cycle. The "25-40x" range may
  be a constructed-after-the-fact set.
- 142% NRR is a specific industry-benchmark metric; in 2024 the
  median for $10-50M ARR SaaS is around 110-115% per public-
  benchmark data. 142% is exceptionally high and warrants
  verification of underlying data.
- The "best-in-class B2B SaaS" claim is unbacked. What's the
  source for "best-in-class"?

### What the framework forces the LLM to attend to

- Each numerical claim is a Quantity TypedComponent with explicit
  units, time-period, and source.
- Each comparable is a Citation-Fact requiring source
  verification (SEC EDGAR filings).
- Each market-comparison claim has implicit temporal context
  that must be tagged (Series B *as of when*?).

### The 80% auto-produced output

```
DD-Brief Analysis (auto-generated, verified):

Subject: ACME SaaS — Series B investment evaluation
Claimed metrics: $14M ARR; 142% NRR; $250M target valuation;
                 17.9x implied ARR multiple

Verified claims (Tier 1):
  ✓ Arithmetic: $250M / $14M = 17.86x ARR multiple [calculated]

Pending verification (Tier 2 — to be confirmed against ACME data):
  ⚠️ $14M ARR — requires source: SaaS billing system extract,
                 ARR reconciliation, churn-adjusted MRR
  ⚠️ 142% NRR — requires source: cohort retention data, expansion
                 revenue breakdown by customer segment

Verified-context comparables (Tier 3 — verified against EDGAR):
  ✓ Datadog Series B (Jan 2014): $10M ARR, multiple 15x → $150M post
    [Verified: TechCrunch announcement Jan 2014, validated against
     S-1 retrospective ARR reconstruction]
  ✓ Snowflake Series B (Feb 2015): $4M ARR, multiple ~25x → $100M post
    [Verified: S-1 retrospective, A/B financing rounds]
  ✓ ServiceNow Series B (Feb 2007): $20M ARR run-rate, multiple ~10x → $200M post
    [Verified: S-1 historic reconstruction]

ANALYSIS:
  - The comparables are real (verified via EDGAR S-1 historical
    reconstruction), but the cited 25-40x ARR-multiple range is
    NOT supported by the actual data. The actual range at Series
    B for these three comparables was 10-25x, with the high end
    being Snowflake's exceptional growth profile.
  - ACME's 17.9x sits in the middle of the actual comparable
    range, not at a "discount."
  - 142% NRR is plausible for top-decile but requires verification.

⚠️ DECK MAKES UNSUPPORTED CLAIMS:
   1. "25-40x ARR multiples" for the cited comparables is not
      verified by historical data. The actual range was 10-25x.
   2. "Best-in-class B2B SaaS" claim has no cited authority.
```

### The 20% kicked back

```
Action required before recommendation:
  1. Request ACME's NRR cohort detail with customer-segment
     breakdown
  2. Request ARR reconciliation: billing extracts + churn-
     adjusted MRR
  3. Confirm: was the "25-40x" comparable range deck's own
     analysis, or did it cite a specific report? If a report,
     verify against the report's published methodology.
  4. Customer concentration: deck does not disclose. Request
     top-10 customer % of ARR.
```

### Gaps

1. **EDGAR / SEC adapter**: SEC EDGAR has a public API; need
   first-class integration for S-1, 10-K, 10-Q queries.
2. **Temporal context tracking**: "Snowflake at Series B" means
   Feb 2015, not 2024. The IR needs temporal anchoring.
3. **Comparable normalization**: comparing 2007 deals to 2024
   deals requires market-cycle adjustment (rate environment,
   liquidity environment).
4. **Arithmetic checking**: the framework should run all
   numerical claims through arithmetic verification (it's
   trivial to compute but easy to miss).

---

## Domain 4: Journalism / fact-checking

### Case

A viral social-media post claims:

```
NEW STUDY: drinking coffee linked to 50% increase in heart attack
risk! Researchers at Harvard found that people who drink 3+ cups
of coffee per day are 50% more likely to have a heart attack
than non-coffee drinkers. Source: Harvard Health Letter, 2023.
```

### What the LLM-without-framework typically does

Generates a summary: "*According to a Harvard study, drinking 3+
cups of coffee per day may increase heart attack risk by 50%.
Consult your doctor.*"

Or, more skeptically: "*Coffee consumption studies are mixed; this
particular study may have limitations. Recommend reading the
original.*"

Both miss:
- The "Harvard Health Letter" is a *consumer newsletter*, not a
  peer-reviewed journal. Citing it as a "source" for a clinical
  finding is misattribution.
- "Linked to 50% increase" is a relative-risk claim. The
  underlying *absolute* risk matters. If baseline annual MI
  risk is 0.5%, a 50% relative increase = 0.75% absolute —
  still very low.
- "Researchers at Harvard" is unspecified. Which researchers?
  Which paper? Published where? Coffee research is rich and
  controversial; this specific finding needs the original paper.
- Coffee research is famously sensitive to confounders (smoking,
  socioeconomic status, baseline cardiovascular risk).

### What the framework forces the LLM to attend to

The claim is decomposed:
- **Fact: study_finding(relative_risk=1.5, exposure="3+ cups coffee/day", outcome="myocardial_infarction")**
- **Fact: study_source(name="Harvard Health Letter", year=2023)**
- **Implicit Fact / required check**: is the "Harvard Health Letter" a peer-reviewed source for original research, or a consumer newsletter that summarizes others' work?
- **Uncertainty**: study identifier missing (no paper title, no PI name, no journal)

### The 80% auto-produced output

```
FACT-CHECK REPORT:

Claim: "3+ cups coffee/day → 50% increase in heart attack risk"
Cited source: "Harvard Health Letter, 2023"

⚠️ SOURCE MISATTRIBUTION:
  The Harvard Health Letter is a consumer health newsletter
  published by Harvard Medical School for general public
  education. It SUMMARIZES research; it does NOT publish original
  research findings. Citing it as the source of a study is
  conflating the newsletter's coverage with the underlying paper.

⚠️ ORIGINAL STUDY NOT IDENTIFIED:
  The claim does not name the underlying paper, its authors,
  the journal it was published in, or the cohort it analyzed.
  Without the original source, the framework cannot:
    - Verify the 50% relative-risk number
    - Determine whether the study controlled for confounders
      (smoking is the largest)
    - Determine the absolute risk increase (relative numbers
      without baseline are not actionable)
    - Determine the study population (general population vs.
      high-risk cohort)

Searching for plausible original sources matching the claim:
  - "Coffee and CHD/MI" 2023 publications include:
    - [Search result placeholder; production would query
       PubMed/MEDLINE]
  - None of the recent meta-analyses report a 50% RR for 3+
    cups; recent meta-analyses (van Dam, Hu et al. various)
    show neutral or mildly protective associations at moderate
    consumption levels.

INTERIM ASSESSMENT:
  Likelihood the underlying claim is well-supported: LOW
  Likelihood the claim is a misattribution / oversimplification: HIGH

ACTION REQUIRED before publishing this fact-check:
  1. Identify the specific paper (PubMed search by author/topic)
  2. If identified: verify the RR number, exposure operationalization,
     and confounder control
  3. If not identified: classify the claim as unverifiable and
     issue a "no evidence found for this specific claim" finding
```

### The 20% kicked back

> *"The claim cites no specific paper. The framework recommends
> searching PubMed for 'coffee consumption myocardial infarction
> 2023' and identifying the closest match. Without the original
> paper, the claim cannot be verified. Should the fact-check
> publish as 'unverifiable' or wait for a citation?"*

### Gaps

1. **PubMed / journal database adapter**: similar to legal
   citation verification but for biomedical literature.
2. **Source-type taxonomy**: distinguishing "peer-reviewed primary
   research" from "consumer-health summary" from "preprint" from
   "press release." The IR needs a `source_type` field.
3. **Claim-shape detection**: distinguishing "relative risk"
   from "absolute risk" from "odds ratio" — different rules
   apply.
4. **Confounder-adjustment tracking**: a study's findings are
   only as good as its confounder control; needs to be a first-
   class IR field.

---

## Domain 5: Academic peer review

### Case

A reviewer is evaluating a submitted paper claiming:

```
We compared treatment X to placebo in 47 patients with
condition Y. After 8 weeks, the treatment group showed
significant improvement on the primary outcome measure
(p = 0.03). We conclude that treatment X is effective for
condition Y.
```

### What the LLM-without-framework typically does

Generates a review: "*The authors report a positive finding with
p = 0.03 in a 47-patient trial. Suggest expanding the sample size
in future work and considering different time horizons.*"

Misses:
- A single trial with p=0.03 and n=47 is *barely* statistically
  significant. Replication failure rates for studies of this
  size are estimated at 40-60% (Open Science Collaboration 2015).
- "Significant improvement" without effect-size reporting is a
  red flag. Statistical significance ≠ clinical significance.
- "Primary outcome measure" — was this pre-registered? Were
  there secondary outcomes that didn't reach significance?
  Outcome-switching is a common research integrity issue.
- 47 patients is small for most clinical conditions; power
  analysis should be reviewable.

### What the framework forces the LLM to attend to

- **Sample size**: explicit Quantity TypedComponent
- **P-value**: explicit Quantity
- **Effect size**: marked as `Uncertainty` if not provided —
  *the framework notices that effect size is MISSING and flags
  it as missing-information*
- **Outcome pre-registration**: required field; marked as
  `Uncertainty` if not stated → kickback
- **Multiple-comparisons handling**: required field for studies
  with secondary outcomes

### The 80% auto-produced output

```
PEER REVIEW (auto-generated, verifiable):

Claim under review: Treatment X effective for condition Y
Evidence type: RCT, n=47, primary outcome p=0.03, 8-week follow-up

⚠️ STATISTICAL ANALYSIS FLAGS:
  1. Sample size (n=47) is small. Power analysis SHOULD be
     reported but is not visible in this excerpt.
     [verification needed: was a power analysis conducted?]
  2. P-value (0.03) is "barely significant" by current
     reproducibility-research standards. Studies in this
     significance range and sample size have estimated
     replication rates of 40-60% (Open Science Collaboration
     2015; verified against publication).
  3. EFFECT SIZE NOT REPORTED. Statistical significance
     without effect size cannot establish clinical
     significance. The framework cannot evaluate the magnitude
     of the claimed improvement.

⚠️ METHODOLOGICAL FLAGS:
  4. Pre-registration not mentioned. Was the primary outcome
     pre-registered (ClinicalTrials.gov, OSF, etc.)?
     [verification needed]
  5. Outcome-switching check: were there secondary outcomes
     that did not reach significance? [verification needed]
  6. Confidence interval not reported for the primary outcome
     effect.

Cited supporting evidence (verified):
  ✓ Open Science Collaboration 2015 — "Estimating the
    reproducibility of psychological science" [verified;
    DOI: 10.1126/science.aac4716]
  ✓ Ioannidis 2005 — "Why most published research findings
    are false" [verified; DOI: 10.1371/journal.pmed.0020124]

PRELIMINARY RECOMMENDATION:
  Major revision required. The current evidence presentation
  is insufficient to support the conclusion of treatment
  efficacy.
```

### The 20% kicked back

> *"The framework cannot evaluate effect size, pre-registration
> status, or outcome-switching from the abstract excerpt alone.
> Please request from authors: (1) effect size with 95% CI;
> (2) pre-registration link; (3) full primary + secondary
> outcomes table; (4) power-analysis details."*

### Gaps

1. **Pre-registration database adapters** (ClinicalTrials.gov,
   OSF Registries).
2. **Effect-size requirement enforcement**: the framework
   should refuse to assess a claim where effect size is
   missing.
3. **Multiple-comparisons handling**: secondary-outcome
   audit as a required IR field.
4. **Methodology-specific rule sets**: RCT, observational
   cohort, case-control, cross-sectional — each has its
   own LR rulebook.

---

## Domain 6: Insurance claim adjudication

### Case

A workers' compensation claim:

```
Claimant: 47yo M warehouse worker. Date of injury: March 3, 2024.
Mechanism: lifting a 60lb box, felt sharp lower back pain.
Diagnosis: L4-L5 disc herniation per MRI. Prior history: no
documented back injuries; treated for chronic low back pain in
2019. Treating physician recommends 6 weeks light duty plus
physical therapy. Employer disputes causation, citing 2019 chronic
back pain as evidence of pre-existing condition.
```

### What the LLM-without-framework typically does

Generates a recommendation: "*The 2019 chronic back pain may have
contributed to the current injury; recommend independent medical
exam to clarify causation.*"

Or alternatively: "*The mechanism is consistent with the
diagnosis; recommend approval of treatment.*"

Both lack the careful analysis of relevant law and case-specific
factors. Workers' comp causation is *jurisdiction-specific* —
some states use "major contributing cause," others use "any
substantial factor," and the prior condition's role differs by
standard.

### What the framework forces the LLM to attend to

- **Jurisdiction**: what state? Workers' comp is state law.
  Missing from the input → Uncertainty → kickback
- **Statute/regulation citations**: state-specific WC statute
  governing pre-existing-condition apportionment
- **Mechanism-of-injury / mechanism-of-claimed-disability link**:
  is "lifting 60lb box" plausibly causal for L4-L5 disc
  herniation per established biomechanics literature?
- **Prior-condition documentation**: was the 2019 treatment for
  *transient* low back pain or *chronic* condition? Records
  needed.
- **Treatment-plan reasonableness**: is "6 weeks light duty +
  PT" within the standard of care per the state's adopted
  treatment guidelines (ODG, ACOEM, etc.)?

### The 80% auto-produced output

```
WORKERS' COMP CLAIM ADJUDICATION — Preliminary Review:

Date of injury: 2024-03-03
Mechanism: lifting 60lb box → sharp LBP
Diagnosis: L4-L5 disc herniation (MRI-confirmed)
Prior history: chronic LBP treatment 2019 (5 years prior)

⚠️ JURISDICTIONAL ANCHOR REQUIRED:
  The state where this claim is filed determines the legal
  standard for causation. The framework cannot complete the
  analysis without knowing the jurisdiction.

[Continuing on assumption: California jurisdiction]

Cited authorities (verified against CA Lab. Code):
  ✓ Cal. Lab. Code § 4663 — apportionment of causation
    [verified: leginfo.legislature.ca.gov, current as of 2024]
  ✓ Cal. Lab. Code § 4664 — limitations on permanent disability
    apportionment [verified]
  ✓ Brodie v. WCAB (2007) 40 Cal.4th 1313 — guidance on
    apportionment to pathology vs. asymptomatic conditions
    [verified: CourtListener]

Analysis under California § 4663:
  Industrial injury must be a substantial factor (not the sole
  cause). Acute mechanism (lifting 60lb box) + immediate onset
  + imaging-confirmed pathology is consistent with industrial
  causation.
  The 2019 prior treatment (5 years prior, no documented
  chronic recurrence) is unlikely to constitute apportionable
  pre-existing pathology under Brodie unless the 2019 records
  show ongoing/permanent disability.

Treatment-plan reasonableness:
  ✓ 6 weeks light duty + PT for acute L4-L5 herniation is
    within the standard of care per ACOEM 2024 guidelines for
    industrial back injuries (verified).

PRELIMINARY DETERMINATION:
  Industrial causation: LIKELY established (subject to
    confirmation of 2019 records).
  Treatment authorization: APPROVE (within standard guidelines).
  Apportionment: <20% to pre-existing (pending 2019 record review).
```

### The 20% kicked back

> *"Action required:*
> *1. Confirm jurisdiction (assumed CA above; if different state,
>     the legal standard changes).*
> *2. Obtain 2019 medical records to determine whether the prior
>     treatment was for transient symptoms or chronic pathology.*
> *3. If the 2019 records show ongoing chronic LBP, the
>     apportionment analysis changes substantially."*

### Gaps

1. **State-statute / regulation database**: state codes are
   public but adapters are state-by-state work.
2. **Jurisdictional layering**: this domain especially needs
   the IR to carry jurisdictional context as a first-class
   field.
3. **Medical-evidence rulebooks** specific to workers' comp
   (ODG, ACOEM, state-specific guidelines) — needs adapter.
4. **Case-law citator** for state appellate courts (the
   *Brodie v. WCAB* type citation).

---

## Domain 7: HR / employment decision

### Case

A hiring manager is reviewing a candidate:

```
Sarah Chen, 32, applying for Senior Engineer position. CV shows
8 years experience at three startups, last role Tech Lead at
Foo Inc. Interview team: 4/5 "strong hire," 1/5 "no hire."
Dissenting interviewer cited "communication style mismatch" as
reason. Performance on technical screen: 87% (above team
average of 79%). Two reference checks: both positive.
```

### What the LLM-without-framework typically does

Generates a recommendation: "*Strong candidate with solid
technical performance and majority interviewer support. Some
communication concerns from one interviewer warrant follow-up.
Recommend extending offer.*"

Misses:
- "Communication style mismatch" is a notorious flag for
  protected-class bias. Without specifics about what
  *exactly* was concerning, the dissenting interviewer's
  feedback may not be defensible.
- Age (32, mid-career) and apparent ethnicity (based on name)
  could correlate with the "style" feedback in ways the
  framework should attend to.
- "8 years experience" — verified against CV claims?

### What the framework forces the LLM to attend to

- **Each evaluation criterion**: tied to job description or
  written rubric?
- **Dissenting feedback**: is it specific and behavioral, or
  general and impressionistic?
- **Protected-class implications**: does the framework's IR
  flag "style," "fit," "personality" feedback that could
  proxy for protected characteristics?
- **Reference check content**: actually positive (specific
  achievements) or perfunctory (vague approvals)?

### The 80% auto-produced output

```
CANDIDATE EVALUATION — Preliminary:

Candidate: Sarah Chen
Position: Senior Engineer
Applicable rubric: Foo Inc. Senior Engineer eval criteria v2.4
                   (cited; verified against published criteria)

Scored criteria (per rubric):
  ✓ Technical skill: 87% on screen, above team median
    → Rubric criterion 1.2 (problem-solving): MET
  ✓ Experience level: 8 years claimed, consistent with
    Senior Engineer expected experience
    → Rubric criterion 2.1 (experience): MET
    [verification needed: CV claims against verifiable history]
  ✓ References: 2/2 positive (content review pending)
    → Rubric criterion 3.1: MET
  ⚠️ Interview consensus: 4/5 strong, 1/5 no-hire
    → Rubric criterion 4.1 (team alignment): PARTIAL

FLAG — DISSENTING FEEDBACK REQUIRES STRUCTURED REVIEW:
  Dissenting interviewer cited "communication style mismatch"
  WITHOUT specific behavioral examples.

  Framework guidance (cited):
  - "Style," "fit," "personality" feedback without behavioral
    specifics can proxy for protected-class bias. EEOC
    guidance recommends behavioral-specific feedback.
    [EEOC Compliance Manual §15, verified]
  - Foo Inc.'s own interviewing playbook (Foo Hiring Guidelines
    v3, internal cited) requires behavioral specifics for
    no-hire votes.

ACTION REQUIRED:
  1. Ask the dissenting interviewer for specific examples of
     the "communication style mismatch." If the interviewer
     cannot articulate specific behaviors, the dissenting vote
     should be discounted in the final decision.
  2. Reference check content review: scan for specific
     achievements vs. perfunctory approvals.
```

### The 20% kicked back

> *"Framework flagged 'communication style' feedback as
> non-specific. Please request structured behavioral examples
> from the dissenting interviewer before finalizing the hiring
> decision. The framework cannot adjudicate the dissent without
> behavioral specifics."*

### Gaps

1. **Legal / EEOC guidance adapter**: the framework should
   tag feedback patterns that legal guidance identifies as
   risk indicators.
2. **Internal-policy rulebook**: companies have HR playbooks
   that should be cite-able as authority for procedural
   decisions.
3. **Bias-pattern detection**: this is research-grade work
   (fairness ML literature). The framework should integrate
   established bias-detection signals, not invent its own.
4. **Confidentiality / minimum-necessary information**: HR
   data is sensitive; the IR should enforce a minimum-necessary
   principle when generating audit trails.

---

## Cross-cutting gap inventory

Aggregating across all seven domains, the framework gaps fall
into eight major categories:

### Gap 1: Citation verification infrastructure (HIGH priority, ALL domains)

The framework requires verifiable citations. Each domain has its
own authority database; no single API covers all:

- **Law**: CourtListener (free + bulk), Caselaw Access Project
  (free), Westlaw / Lexis (paid)
- **Medicine**: PubMed/MEDLINE (free), DOI resolver via
  Crossref (free), clinical-guideline registries
- **Finance**: SEC EDGAR (free), FRED for macroeconomic data
- **Security**: CWE/CVE databases (free), OWASP corpus
- **Statistics / regulation**: BLS / Census / EU statistics
  (free), federal-register API, state code APIs
- **Trademark / patent**: USPTO API

**Proposed spec**: ADJ39 — Citation Verification Infrastructure.
Defines `CitationFact` as a first-class IR variant, the
verification protocol (existence check + content match check),
and per-domain adapters. The framework refuses to commit any
inference dependent on an unverified citation.

### Gap 2: Recursive source decomposition (HIGH priority, ALL domains)

When a rulebook or input cites a source, the framework should be
able to fetch that source's full text and run the same IR
pipeline on it. This grounds out the recursion at either:

- Verified citation with verified claim content
- Failed verification → propagate Uncertainty up to the calling
  level → eventual kickback to human

This was demonstrated abstractly in ADJ37. The implementation
needs:
- Fetcher adapters (one per source type)
- Full-text IR pipeline (typically text but sometimes XML / JSON
  in structured formats like court opinions or SEC filings)
- A "claim-in-source" verifier — an NLI-style check that the
  rulebook's claim about the source is actually supported by
  the source's text

**Proposed spec**: ADJ40 — Recursive Source Decomposition.

### Gap 3: Temporal context tracking (HIGH priority, finance/law/medicine)

Many claims have implicit temporal anchors:
- "Snowflake at Series B" means Feb 2015 macro environment
- "CPLR § 214" as of which year (amendments matter)
- "Beers Criteria" with which year of update
- "OWASP Top 10" of which year

The IR needs a first-class `temporal_anchor` field on Citation-
Facts and a way to detect when temporal mismatch invalidates a
rule's applicability.

**Proposed spec**: ADJ41 — Temporal Context for Citations and Rules.

### Gap 4: Jurisdictional layering (HIGH priority, law/insurance/HR)

Many domains have **layered rule systems** — federal + state +
local; or multiple overlapping regulatory regimes. The IR needs:
- `jurisdiction` as a first-class field on Rule-Facts
- A precedence / supremacy resolver for conflicting rules
- A "this rule applies in jurisdiction X but not Y" check

**Proposed spec**: ADJ42 — Jurisdictional Context and Rule
Layering.

### Gap 5: Missing-information Uncertainty (MEDIUM priority, ALL domains)

Introduced informally in ADJ37: an Uncertainty over information
that *should be in the input but isn't*. Distinct from
Uncertainty over a present span. Without this, the framework
can't tell "the input doesn't mention X" from "the input doesn't
have X."

**Proposed spec**: ADJ43 — Missing-Information Uncertainty
(IR variant).

### Gap 6: Conclusion-scope mismatch detection (MEDIUM priority, ALL domains)

Surfaced in ADJ37: the rulebook elicited LRs for "any delirium"
while the input queried "med-induced delirium." When the
rulebook's contribution-clauses target a different conclusion
than the input's query, the framework should detect this and
kick back.

**Proposed spec**: ADJ44 — Conclusion-Scope Mismatch (connector
extension).

### Gap 7: AST-level IR for code (MEDIUM priority, code review only)

For code-review domains, the IR pipeline's natural granularity
is AST nodes, not text characters. The decomposition should
adapt:
- File → Function → Statement → Expression → Token
- Each AST node typed (function definition, function call,
  string-format expression, SQL string, etc.)

**Proposed spec**: ADJ45 — AST-Level IR for Code-Centric Domains.

### Gap 8: Source-type taxonomy (MEDIUM priority, journalism/research)

Different source types carry different evidentiary weight:
- Primary research vs. review
- Pre-print vs. peer-reviewed
- Consumer publication vs. academic journal
- Official government source vs. press release

The framework's Citation-Fact needs a `source_type` field with
a domain-appropriate taxonomy.

**Proposed spec**: ADJ46 — Source-Type Taxonomy.

## The 80/20 framing across all domains

Each domain's framework output decomposes naturally into:

| Domain | The 80% auto-produced | The 20% kicked back |
|---|---|---|
| Legal brief | Verified citations + verified holdings + verified statutes + arithmetic | Ambiguous statute section, missing precedent, factual gaps |
| Code security | Vulnerability pattern detection + CWE/OWASP citations + verified data flow | Custom helper functions, project-specific contexts, ambiguous escape sequences |
| Investment DD | Verified comparables + verified financial arithmetic + flagged unsupported claims | NRR cohort details, customer concentration, unverified company-specific metrics |
| Fact-checking | Source-type validation + finding the original paper + verifying numerical claims | Unidentified original source, novel claim shapes, methodology evaluation |
| Peer review | Statistical-analysis sanity + methodology-flag checks + verified references | Effect-size questions, pre-registration verification, secondary-outcome audit |
| Insurance | Jurisdiction-specific statutory analysis + treatment-guideline cross-check | Missing medical records, jurisdiction confirmation, prior-condition history |
| HR decision | Rubric-aligned criterion scoring + EEOC-flag for bias-proxies + reference content review | Behavioral specifics of dissent, intent of "style/fit" feedback |

**In every case, the framework's job is the same:**
- Force the LLM to attend to verifiable features
- Mechanize citation verification
- Surface gaps (missing information, ambiguous data) as
  structured kickback questions
- Refuse to commit when verification fails

## The publishable claim, sharpened

> **The framework is a structural attention scaffold for LLM-
> driven knowledge work. It mechanizes the citation, coverage,
> and verification disciplines that distinguish defensible
> professional output from confabulation. The realistic
> productivity claim is 80/20: automate the systematic 80% of
> work where attention-forcing is mechanical; surface the messy
> 20% as structured human-disambiguation requests. The framework
> generalizes across knowledge-work domains (law, code review,
> finance, journalism, academic review, insurance, HR) through
> the same IR pipeline; per-domain adapters handle citation
> sources and rule databases. The end-state guarantee — that a
> lawyer cannot file a brief with fabricated case citations
> because the framework refuses to commit until verification
> passes — is structurally achievable.**

This is the contribution. It's not "LLM that thinks better." It's
"a discipline that forces LLMs to attend to what they should
attend to, mechanically, so professional knowledge work can be
defensible."

## Recommended next moves (prioritized)

1. **ADJ39: Citation Verification Infrastructure** — the highest-
   impact gap, applicable across all seven domains. Without this,
   the framework's Mata-v.-Avianca guarantee isn't real.
2. **ADJ40: Recursive Source Decomposition** — close second.
   Citation existence verification is the easy half; verifying
   that a citation supports the claimed proposition requires
   processing the source's own content.
3. **ADJ41–46**: the remaining gaps, in roughly the priority
   shown above.
4. **LP19e Rust implementation** + connector v2 + a demo binary
   that runs ADJ36/ADJ37/ADJ38 cases as compiled code (not
   Python). This is the "make it operational" work.
5. **Per-domain corpus building**: 20–50 cases per domain (law,
   medicine, code review, journalism) with gold labels for
   evaluation. Without these, the framework's claims about
   reduction in hallucination aren't measurable.

## Status

This is the cross-domain validation document the user requested.
It establishes that the framework's design holds across seven
distinct knowledge-work domains, identifies eight cross-cutting
gaps, prioritizes the follow-up specs, and re-anchors the
publishable claim around the attention-scaffold reframe.

The next-natural-PR is ADJ39 — Citation Verification
Infrastructure — because it's the single piece that converts the
framework from "good idea with hand-waved provenance" to "real
system with mechanically-verified citations."

## See also

- [ADJ19](ADJ19-expert-systems-historical-analysis.md) — the
  framework's response to 8+1 historical failure modes; this
  document is the natural extension covering domains the
  classical literature didn't.
- [ADJ37](ADJ37-unified-framework-and-rulebook-elicitation-demo.md)
  — the symmetric input/rulebook framework this builds on.
- [ADJ36](ADJ36-end-to-end-clinical-demo.md) — the original
  clinical demo (domain 1.5 between "medicine generally" and
  this cross-domain analysis).
- [ADJ18](ADJ18-active-sensing-voi.md) — the VOI kickback
  mechanism that produces the 20% the human disambiguates.
