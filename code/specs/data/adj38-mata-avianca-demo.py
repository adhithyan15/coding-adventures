#!/usr/bin/env python3
"""ADJ38 — Mata v. Avianca scenario demonstration.

Shows the framework refusing to commit a legal brief because
Citation-Facts fail verification. This is the canonical
attention-failure case the framework must structurally prevent.

In a production implementation, the verification calls would hit
CourtListener / Caselaw Access Project / Westlaw APIs. In this
proof, the verification is mocked — but the API shape, the
decision logic, and the audit-trail output are realistic.

Run: python3 adj38-mata-avianca-demo.py
"""

import dataclasses
from typing import Optional


# ---------------------------------------------------------------------------
# The IR
# ---------------------------------------------------------------------------

@dataclasses.dataclass
class CitationFact:
    """A legal citation extracted from the brief."""
    citation_text: str                # original cited string from brief
    parties: str                      # plaintiff v. defendant
    reporter: str                     # e.g., "F.3d"
    volume: int
    page: int
    court: str                        # e.g., "2d Cir."
    year: int
    claimed_proposition: str          # what the brief says this case stands for


@dataclasses.dataclass
class StatuteCitation:
    """A statute citation extracted from the brief."""
    citation_text: str                # e.g., "CPLR § 214(5)"
    code: str                         # e.g., "NY CPLR"
    section: str                      # e.g., "214(5)"
    claimed_scope: str                # what the brief says this section governs


@dataclasses.dataclass
class FactualAssertion:
    date: str
    description: str


@dataclasses.dataclass
class VerificationResult:
    status: str                       # "verified", "not_found", "wrong_holding",
                                      # "court_mismatch", "wrong_scope", etc.
    detail: str
    source_url: Optional[str] = None


# ---------------------------------------------------------------------------
# The brief text (as the framework would receive it)
# ---------------------------------------------------------------------------

BRIEF_TEXT = """The Court should dismiss this action because Plaintiff
filed the complaint more than two years after the cause of action
accrued, in violation of CPLR § 214(5). The Second Circuit held in
Varghese v. China Southern Airlines, 925 F.3d 1339 (2d Cir. 2019),
that statute-of-limitations defenses are non-waivable in diversity
actions. Plaintiff's claim accrued on June 15, 2021, when the breach
was discovered; this action was filed on September 22, 2023, well
outside the two-year window."""


# ---------------------------------------------------------------------------
# Step 1 — IR extraction (Claude as Role::Extractor, hand-traced for demo)
# ---------------------------------------------------------------------------

CITATIONS = [
    CitationFact(
        citation_text="Varghese v. China Southern Airlines, 925 F.3d 1339 (2d Cir. 2019)",
        parties="Varghese v. China Southern Airlines",
        reporter="F.3d",
        volume=925,
        page=1339,
        court="2d Cir.",
        year=2019,
        claimed_proposition="statute-of-limitations defenses are non-waivable in diversity actions",
    ),
]

STATUTES = [
    StatuteCitation(
        citation_text="CPLR § 214(5)",
        code="NY CPLR",
        section="214(5)",
        claimed_scope="two-year limitations period for breach of contract claims",
    ),
]

FACTS = [
    FactualAssertion(
        date="2021-06-15",
        description="cause of action accrued (breach discovered)",
    ),
    FactualAssertion(
        date="2023-09-22",
        description="action filed",
    ),
]


# ---------------------------------------------------------------------------
# Step 2 — Mocked verification adapters
# (In production: CourtListener, Caselaw Access Project, Westlaw,
# NY State legislative database, etc.)
# ---------------------------------------------------------------------------

def verify_case_citation(c: CitationFact) -> VerificationResult:
    """Existence check + holding check via mocked CourtListener.

    A real implementation would:
      1. Query CourtListener API with the citation
      2. If 200/found: retrieve the opinion text
      3. Run NLI check: does the opinion text support
         `c.claimed_proposition`?
      4. Return the verification status + source URL.

    Mocked here because we have no network in this demo. Each
    mocked response reflects what a real CourtListener query
    would return.
    """
    if c.parties == "Varghese v. China Southern Airlines":
        # This case does not exist. ChatGPT confabulated it in
        # the real Mata v. Avianca filing. CourtListener returns
        # no match. Caselaw Access Project: no match. Westlaw:
        # no match. The framework returns verification_failed.
        return VerificationResult(
            status="not_found",
            detail=(
                "No case matches the citation "
                f"'{c.citation_text}'. "
                "CourtListener, Caselaw Access Project, and "
                "Westlaw return no result. The F.3d 925 volume "
                "exists, but page 1339 in that volume does NOT "
                "contain a case with parties matching "
                f"'{c.parties}'. The Second Circuit's published "
                "opinions for 2019 do not include a case with "
                "these parties."
            ),
            source_url=None,
        )
    # Generic fallback for any other citation we might want to test
    return VerificationResult(
        status="not_found",
        detail=f"Citation '{c.citation_text}' not found in mocked database.",
    )


def verify_statute_citation(s: StatuteCitation) -> VerificationResult:
    """Statute existence + claimed-scope check.

    A real implementation queries the state's legislative
    database (CALI, NY State Senate, etc.).
    """
    if s.code == "NY CPLR" and s.section == "214(5)":
        # The section exists, but its scope is NOT contract
        # claims. It's negligence resulting in personal injury,
        # including medical/dental/podiatric malpractice. The
        # brief's claimed scope is wrong.
        return VerificationResult(
            status="wrong_scope",
            detail=(
                "NY CPLR § 214(5) governs personal injury "
                "claims arising from negligence (including "
                "medical, dental, and podiatric malpractice), "
                "NOT breach of contract. For breach of a "
                "written contract, the applicable statute of "
                "limitations is NY CPLR § 213(2) (six years). "
                "The brief's reliance on § 214(5) as the "
                "limitations period for a contract claim is "
                "incorrect."
            ),
            source_url="https://www.nysenate.gov/legislation/laws/CVP/214",
        )
    return VerificationResult(
        status="not_found",
        detail=f"Statute '{s.citation_text}' not found in mocked database.",
    )


# ---------------------------------------------------------------------------
# Step 3 — Run the verification chain
# ---------------------------------------------------------------------------

def evaluate_brief():
    print("=" * 72)
    print("ADJ38 — Mata v. Avianca Demonstration")
    print("=" * 72)
    print()
    print("BRIEF UNDER REVIEW:")
    print()
    for line in BRIEF_TEXT.split("\n"):
        print(f"  {line}")
    print()
    print("=" * 72)
    print("Framework's Citation-Fact verification chain")
    print("=" * 72)
    print()

    failures = []

    # --- Case citations ---
    print("--- CASE CITATIONS ---")
    for c in CITATIONS:
        result = verify_case_citation(c)
        marker = "✓" if result.status == "verified" else "❌"
        print(f"\n{marker} {c.citation_text}")
        print(f"   Status: {result.status}")
        if result.detail:
            for line in result.detail.split(". "):
                if line.strip():
                    print(f"     {line.strip()}.")
        if result.status != "verified":
            failures.append(("case_citation", c.citation_text, result.detail))

    # --- Statute citations ---
    print("\n--- STATUTE CITATIONS ---")
    for s in STATUTES:
        result = verify_statute_citation(s)
        marker = "✓" if result.status == "verified" else "❌"
        print(f"\n{marker} {s.citation_text}")
        print(f"   Status: {result.status}")
        if result.detail:
            for line in result.detail.split(". "):
                if line.strip():
                    print(f"     {line.strip()}.")
        if result.status != "verified":
            failures.append(("statute_citation", s.citation_text, result.detail))

    # --- Factual assertions (arithmetic, dates) ---
    print("\n--- FACTUAL ASSERTIONS ---")
    from datetime import date
    accrual = date.fromisoformat("2021-06-15")
    filing = date.fromisoformat("2023-09-22")
    elapsed_days = (filing - accrual).days
    elapsed_years = elapsed_days / 365.25
    print()
    print(f"  Accrual:  {accrual.isoformat()}")
    print(f"  Filing:   {filing.isoformat()}")
    print(f"  Elapsed:  {elapsed_days} days = {elapsed_years:.2f} years")
    print(f"  Arithmetic: 2 years 3 months 7 days (matches brief's claim of >2 years)")
    print()

    # --- Decision ---
    print("=" * 72)
    if not failures:
        print("✓ DECISION: ALL CITATIONS VERIFIED. Brief may proceed.")
    else:
        print("❌ DECISION: DO NOT SHIP. Brief contains unverified citations.")
        print()
        print(f"Number of verification failures: {len(failures)}")
        print()
        print("Action required before brief can be filed:")
        for i, (kind, citation, detail) in enumerate(failures, 1):
            print(f"  {i}. {kind}: {citation}")
            print(f"     Issue: {detail[:200]}...")
            print()
        print("The framework refuses to commit this brief to the court.")
        print("The user (junior associate) must either:")
        print("  - Replace the unverified citation with a real authority")
        print("    that supports the claimed proposition.")
        print("  - Reformulate the brief's argument to rely only on")
        print("    citations that pass verification.")
        print("  - Remove the unsupported claim entirely.")
        print()
        print("This is the Mata v. Avianca failure mode the framework")
        print("structurally prevents.")
    print("=" * 72)
    print()


# ---------------------------------------------------------------------------
# Step 4 — Audit trail (what the lawyer would see)
# ---------------------------------------------------------------------------

def print_audit_trail():
    print("=" * 72)
    print("AUDIT TRAIL")
    print("=" * 72)
    print()
    print("Brief provenance:")
    print(f"  - Source text: {len(BRIEF_TEXT)} bytes, 1 paragraph")
    print(f"  - Citation-Facts extracted: {len(CITATIONS)}")
    print(f"  - Statute-Citation-Facts extracted: {len(STATUTES)}")
    print(f"  - Factual-Assertion-Facts extracted: {len(FACTS)}")
    print()
    print("Verification calls made (mocked):")
    print(f"  - CourtListener API queries: {len(CITATIONS)}")
    print(f"  - NY State legislative database queries: {len(STATUTES)}")
    print(f"  - Date arithmetic: 1 computation")
    print()
    print("Verification outcomes (anchored to authoritative sources):")
    print()
    print("  Case: Varghese v. China Southern Airlines, 925 F.3d 1339 (2d Cir. 2019)")
    print("    → Verification: NOT FOUND")
    print("    → Authority queried: CourtListener (free, open-access)")
    print("    → Confidence in non-existence: HIGH")
    print("       (no case with these parties + reporter + court + year)")
    print()
    print("  Statute: NY CPLR § 214(5)")
    print("    → Verification: WRONG SCOPE")
    print("    → Authority queried: nysenate.gov / NY State Senate")
    print("    → § 214(5) exists but governs personal injury negligence,")
    print("       not breach of contract. Brief should cite § 213(2).")
    print()
    print("This trail is reproducible: every verification step can be")
    print("re-run from the same Citation-Fact + adapter pair, producing")
    print("the same answer. A reviewing partner or opposing counsel can")
    print("independently verify by clicking each source_url.")


def main():
    evaluate_brief()
    print_audit_trail()


if __name__ == "__main__":
    main()
