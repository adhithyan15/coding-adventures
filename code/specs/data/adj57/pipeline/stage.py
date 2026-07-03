#!/usr/bin/env python3
"""ADJ58 — the universal stage contract.

There is no privileged stage. Every transform in the framework — normalize,
decompose, derive, ground, aggregate — takes an input and must PROVE it accounted
for 100% of that input: every used unit cites the output it produced, every
discarded unit carries a reason, and the proof is appended to one composed,
auditable Trail. A stage that drops part of its input silently is a hole; the gate
refuses it.

Two input shapes, one contract:

  - TEXT inputs (a case, a source page): the proof is a PARTITION of the string.
    Concatenating the segments in order must reproduce the input byte-for-byte.
    Each segment is `used` (cites what it produced) or `discard` (carries a reason).

  - ELEMENT inputs (a list of facts, a set of grounded LRs): the proof partitions
    the SET of element ids. Every id is either `used` (cites its output) or
    `discard` (carries a reason); used ∪ discard == all ids, disjoint.

`clean` means: covered AND every used cites a `produced` AND every discard a
`reason`. `Trail.ok()` means every stage is clean — the byte-trail is unbroken
from raw input to final output.
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field


@dataclass
class Coverage:
    stage: str
    kind: str  # 'text' | 'elements'
    covered: bool
    clean: bool
    n_input: int
    n_used: int
    n_discard: int
    used: list = field(default_factory=list)
    discards: list = field(default_factory=list)
    detail: dict = field(default_factory=dict)


def gate_text(stage: str, input_text: str, segments: list[dict]) -> Coverage:
    """segments: ordered [{text, kind:'used'|'discard', produced?:..., reason?:...}].
    Covered iff concatenating segment.text reproduces input_text exactly."""
    recon = "".join(s["text"] for s in segments)
    covered = recon == input_text
    detail: dict = {}
    if not covered:
        i = 0
        m = min(len(recon), len(input_text))
        while i < m and recon[i] == input_text[i]:
            i += 1
        detail = {"first_divergence": i, "expected": input_text[i:i + 50], "got": recon[i:i + 50]}
    used = [s for s in segments if s.get("kind") == "used"]
    disc = [s for s in segments if s.get("kind") == "discard"]
    clean = covered and all(s.get("produced") for s in used) and all(s.get("reason") for s in disc)
    return Coverage(stage, "text", covered, clean, len(input_text), len(used), len(disc),
                    used=used, discards=disc, detail=detail)


def partition_text_by_used(input_text: str, used: list[dict], discard_reason: str) -> list[dict]:
    """Build a text partition from a list of verbatim `used` substrings: locate each
    (in order, non-overlapping) and fill the gaps between them as `discard` segments
    with a default reason. Used for stages (like ground) where the agent gives the
    load-bearing quotes and the surrounding page is context to discard-with-reason."""
    segments: list[dict] = []
    pos = 0
    for u in used:
        q = u["text"]
        idx = input_text.find(q, pos)
        if idx < 0:
            # quote not verbatim in input -> a provenance break; record it as such
            segments.append({"text": "", "kind": "discard", "reason": f"BROKEN: quote not found verbatim: {q[:60]!r}"})
            continue
        if idx > pos:
            segments.append({"text": input_text[pos:idx], "kind": "discard", "reason": discard_reason})
        segments.append({"text": q, "kind": "used", "produced": u.get("produced")})
        pos = idx + len(q)
    if pos < len(input_text):
        segments.append({"text": input_text[pos:], "kind": "discard", "reason": discard_reason})
    return segments


def gate_elements(stage: str, input_ids: list[str], used: list[dict], discards: list[dict]) -> Coverage:
    """used: [{id, produced}], discards: [{id, reason}]. Covered iff used ∪ discard
    == set(input_ids) with no id in both and none missing."""
    uids = [u["id"] for u in used]
    dids = [d["id"] for d in discards]
    seen = set(uids) | set(dids)
    allids = set(input_ids)
    missing = allids - seen           # silently dropped -> the hole
    extra = seen - allids             # cited an id that wasn't in the input
    overlap = set(uids) & set(dids)   # both used and discarded
    covered = not missing and not extra and not overlap
    clean = covered and all(u.get("produced") for u in used) and all(d.get("reason") for d in discards)
    detail = {"missing": sorted(missing), "extra": sorted(extra), "overlap": sorted(overlap)}
    return Coverage(stage, "elements", covered, clean, len(allids), len(used), len(discards),
                    used=used, discards=discards, detail=detail)


class Trail:
    """The one composed, auditable log. Every stage appends its Coverage; the trail
    is ok iff every stage is clean (the byte-trail is unbroken end to end)."""

    def __init__(self) -> None:
        self.stages: list[Coverage] = []

    def record(self, cov: Coverage) -> Coverage:
        self.stages.append(cov)
        return cov

    def ok(self) -> bool:
        return bool(self.stages) and all(c.clean for c in self.stages)

    def holes(self) -> list[str]:
        out = []
        for c in self.stages:
            if not c.covered:
                out.append(f"{c.stage}: NOT COVERED ({c.detail})")
            elif not c.clean:
                bad_u = [u for u in c.used if not u.get("produced")]
                bad_d = [d for d in c.discards if not d.get("reason")]
                out.append(f"{c.stage}: covered but unclean ({len(bad_u)} used w/o citation, {len(bad_d)} discard w/o reason)")
        return out

    def summary(self) -> str:
        lines = ["AUDIT TRAIL — every stage must account for 100% of its input:"]
        for c in self.stages:
            mark = "OK " if c.clean else ("!! " if not c.covered else " ~ ")
            if c.kind == "text":
                ub = sum(len(u["text"]) for u in c.used)
                db = sum(len(d["text"]) for d in c.discards)
                lines.append(f"  [{mark}] {c.stage:14s} text  {c.n_input:5d} bytes = "
                             f"{c.n_used} used ({ub}b) + {c.n_discard} discard ({db}b)")
            else:
                lines.append(f"  [{mark}] {c.stage:14s} elem  {c.n_input:5d} units = "
                             f"{c.n_used} used + {c.n_discard} discard"
                             + (f"  MISSING={c.detail['missing']}" if c.detail.get("missing") else ""))
        lines.append(f"  => trail {'UNBROKEN (every stage byte-accounted)' if self.ok() else 'HAS HOLES: ' + '; '.join(self.holes())}")
        return "\n".join(lines)

    def to_json(self) -> str:
        return json.dumps([{
            "stage": c.stage, "kind": c.kind, "covered": c.covered, "clean": c.clean,
            "n_input": c.n_input, "n_used": c.n_used, "n_discard": c.n_discard,
            "discards": [{"reason": d.get("reason"), **({"id": d["id"]} if "id" in d else {"bytes": len(d.get("text", ""))})} for d in c.discards],
            "detail": c.detail,
        } for c in self.stages], indent=2)
