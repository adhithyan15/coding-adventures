#!/usr/bin/env python3
"""harness.py — the generic cold-path grounding harness + recursive source objects.

The single on-ramp by which any fact enters the CAS: spider → byte-provenance →
adversarial gate → commit. This module is the REUSABLE core (G0) under the
per-artifact gates (e.g. diagnosis/organisms/organism_id_ground.py), plus the new
recursive **source grounding**:

  NOTHING ON BLIND TRUST. A fact cites a source with a byte-quote. But a quote can
  be cherry-picked or misread (the G1 run caught a "community" prior whose quote was
  actually about *nosocomial* infection). So the SOURCE ITSELF is fetched,
  DECOMPOSED into byte-provenanced claims, and committed to the CAS as a *source
  object*; a fact's citation is then VERIFIED against the decomposed source — does
  the source actually contain (and support) what the fact implies? — rather than
  trusted. Sources a source itself cites are decomposed too (bounded recursion),
  so the provenance is a Merkle graph: fact → source object → cited source object.

This file holds the deterministic pieces (gate verdicts, the source-object model +
CAS read/write, citation verification, the system-wide provenance ledger). The
spidering itself is the Workflow scripts in grounding/workflows/.
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import asdict, dataclass, field
from pathlib import Path

MYCIN = Path(__file__).resolve().parent.parent
SOURCES_DIR = MYCIN / "cas" / "sources"   # decomposed sources live IN the CAS


# --------------------------------------------------------------------------
# The gate verdict (shared by every artifact's write gate; mirrors cas_build.py).
# --------------------------------------------------------------------------
def gate(spider_status: str) -> tuple[str, str]:
    """(verdict, trust) for a gradeable clause given its reconciled spider status.
    `grounded` (a primary source affirms it AND re-extraction-stable) → ACCEPT at
    trust authoritative; everything else (`direction_only`/`refuted`/`ungrounded`/
    missing) → FLAG, kept at trust `inferred`, NEVER silently used as authoritative."""
    return ("ACCEPT", "authoritative") if spider_status == "grounded" else ("FLAG", "inferred")


# --------------------------------------------------------------------------
# Source objects — a decomposed primary source, content-addressed in the CAS.
# --------------------------------------------------------------------------
@dataclass(frozen=True)
class SourceClaim:
    """One byte-provenanced assertion the source makes."""
    id: str
    text: str        # the claim in normalized form
    byte_quote: str   # the VERBATIM span from the source that states it


@dataclass
class SourceObject:
    source_id: str               # canonical id (DOI / PubMed / URL)
    title: str
    resolved_url: str
    claims: list[SourceClaim] = field(default_factory=list)
    cites: list[str] = field(default_factory=list)   # child source ids (recursion)

    def canonical(self) -> str:
        return json.dumps({
            "source_id": self.source_id, "title": self.title, "resolved_url": self.resolved_url,
            "claims": sorted([[c.id, c.text, c.byte_quote] for c in self.claims]),
            "cites": sorted(self.cites),
        }, separators=(",", ":"), ensure_ascii=False)

    def content_hash(self) -> str:
        return hashlib.sha256(self.canonical().encode()).hexdigest()[:16]


def _norm(s: str) -> str:
    """Normalize for robust quote matching: case- and whitespace-insensitive, and
    insensitive to Markdown emphasis (`_`, `*`, backtick) and HTML entities that a
    web fetch injects around the SAME underlying bytes — e.g. the source renders
    `_S. pneumoniae_` / `P&lt;0.001` where the fact quotes `S. pneumoniae` / `P<0.001`.
    Matching is on the prose; the verbatim byte_quote is preserved as stored."""
    s = (s or "")
    s = s.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
    s = re.sub(r"[*_`]", "", s)               # strip Markdown emphasis markers
    s = re.sub(r"[–—]|--", "-", s)   # en/em-dash & "--" → hyphen (range "9–23" == "9--23")
    return re.sub(r"\s+", " ", s).strip().lower()


def write_source_object(obj: SourceObject) -> str:
    """Commit a decomposed source to the CAS (cas/sources/<hash>.json). Returns hash."""
    SOURCES_DIR.mkdir(parents=True, exist_ok=True)
    h = obj.content_hash()
    payload = {"hash": h, **asdict(obj)}
    (SOURCES_DIR / f"{h}.json").write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")
    return h


def load_source_object(h: str) -> SourceObject | None:
    p = SOURCES_DIR / f"{h}.json"
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    return SourceObject(
        source_id=d["source_id"], title=d["title"], resolved_url=d["resolved_url"],
        claims=[SourceClaim(**c) for c in d.get("claims", [])], cites=d.get("cites", []),
    )


# --------------------------------------------------------------------------
# Citation verification — does the source actually SAY what the fact implies?
# --------------------------------------------------------------------------
def verify_citation(fact_byte_quote: str, source: SourceObject) -> dict:
    """Verify a fact's cited byte-quote against the DECOMPOSED source (no blind
    trust). The cited quote must actually appear in the source's decomposed claims
    — i.e. it was really in the source, not hallucinated or cherry-picked out of a
    paraphrase. Returns {verified, matched_claim, reason}.

    This is the deterministic floor ('the quote is genuinely in the source'); a
    deeper 'the quote ENTAILS the implication' check is an adversarial agent pass
    layered on top, but a quote that isn't even in the decomposed source fails here."""
    raw = _norm(fact_byte_quote)
    if not raw:
        return {"verified": False, "core_verified": False, "fragments_matched": 0,
                "fragments_total": 0, "matched_claim": None, "reason": "empty citation"}
    # A cited quote may be a COMPOSITE stitched with an ellipsis ("A … B"). That is
    # not one contiguous span, so each fragment is checked independently against the
    # decomposed source. The FIRST fragment is the load-bearing evidentiary span (the
    # proportion / the morphology); later fragments are bundled context. We report:
    #   verified       — EVERY fragment is present (the whole citation is supported), and
    #   core_verified  — at least the first (load-bearing) fragment is present.
    # A fragment stitched in from elsewhere (or not yet decomposed) makes verified
    # False while core_verified can still hold — so an over-stuffed citation is flagged
    # for fix-up without falsely discrediting the fact's actual evidence.
    frags = [f for f in re.split(r"\s*(?:\.\.\.|…)\s*", raw) if len(f) >= 8] or [raw]

    def hit(frag: str) -> str | None:
        return next((c.id for c in source.claims
                     if (cq := _norm(c.byte_quote)) and (frag in cq or cq in frag)), None)

    hits = [hit(f) for f in frags]
    matched_n = sum(h is not None for h in hits)
    core = hits[0] is not None
    all_ok = matched_n == len(frags)
    if all_ok:
        reason = ("cited quote is present in the decomposed source" if len(frags) == 1
                  else "every fragment of the cited quote is present in the decomposed source")
    elif core:
        reason = (f"core span verified, but {len(frags) - matched_n} of {len(frags)} stitched "
                  "fragments are not in the decomposed source — citation over-reaches (fix up)")
    else:
        reason = ("the decomposed source does NOT contain the cited quote — citation "
                  "unverified (does the source say what was implied?)")
    return {"verified": all_ok, "core_verified": core, "fragments_matched": matched_n,
            "fragments_total": len(frags), "matched_claim": hits[0], "reason": reason}


def source_object_from_record(rec: dict) -> SourceObject:
    """Build a SourceObject from a decompose-source workflow record."""
    g = rec
    return SourceObject(
        source_id=g.get("source_id") or g.get("resolved_url", ""),
        title=g.get("title", ""), resolved_url=g.get("resolved_url", ""),
        claims=[SourceClaim(id=c["id"], text=c.get("text", ""), byte_quote=c["byte_quote"])
                for c in g.get("claims", [])],
        cites=g.get("cites", []),
    )


# --------------------------------------------------------------------------
# System-wide provenance ledger — every fact's status across all artifacts.
# --------------------------------------------------------------------------
def build_ledger(artifacts: list[dict]) -> str:
    """Render the system provenance ledger. `artifacts` is a list of
    {name, path, grounded, flagged, authored_debt, rows:[(clause,status,gate,source,
    source_verified)]}."""
    # Cell values can carry semi-trusted spider text (e.g. a source title); escape the
    # Markdown table metacharacters so a crafted value can't forge columns/rows or
    # otherwise mislead a human auditor reading this ledger.
    def _cell(v: object) -> str:
        return (str(v).replace("\\", "\\\\").replace("|", "\\|").replace("`", "\\`")
                .replace("\r", " ").replace("\n", " "))

    tot_g = sum(a["grounded"] for a in artifacts)
    tot_f = sum(a["flagged"] for a in artifacts)
    tot_d = sum(a["authored_debt"] for a in artifacts)
    out = [
        "# Provenance ledger — MYCIN-2026 (system-wide)",
        "",
        "Nothing is human-authored: every fact enters the CAS only via the cold path",
        "(spider → byte-provenance → adversarial gate). Cited sources are themselves",
        "decomposed into the CAS and citations are VERIFIED against them — nothing on",
        "blind trust. This ledger (generated by the gates via grounding/harness.py)",
        "tracks each fact's status so **authoring debt** is visible and drives to zero.",
        "",
        f"**Totals — grounded: {tot_g} · inferred-flagged: {tot_f} · authored-debt: {tot_d}**",
        "",
    ]
    for a in artifacts:
        out += [f"## {a['name']}  (`{a['path']}`)", "",
                f"grounded: **{a['grounded']}** · flagged: **{a['flagged']}** · "
                f"authored-debt: **{a['authored_debt']}**", "",
                "| clause | status | gate | source | source-verified |",
                "|---|---|---|---|---|"]
        for row in a["rows"]:
            clause, status, verdict, src, sv = (list(row) + ["—"])[:5]
            out.append(f"| `{_cell(clause)}` | {_cell(status)} | {_cell(verdict)} | "
                       f"{_cell(src)} | {_cell(sv)} |")
        out.append("")
    return "\n".join(out) + "\n"
