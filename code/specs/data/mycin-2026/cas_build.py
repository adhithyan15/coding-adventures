#!/usr/bin/env python3
"""cas_build.py - build the content-addressed store of importable adj-lang libraries.

MYCIN-2026 M5. The CAS is NOT a passive blob store: every object is a runnable
`adj-lang` LIBRARY that other libraries `import`. This builder turns the authored
`lib/*.adj` graph into a content-addressed library graph in `cas/objects/`:

  - It content-addresses BOTTOM-UP (leaves first). A library's hash is the
    sha256 of its source *after* its `import "name.adj"` lines are rewritten to
    `import "<hash-of-dependency>.adj"`. So the hash is Merkle-style: editing the
    dictionary changes its hash, which changes every arm that imports it, which
    changes the composed rulebook - immutable, tamper-evident provenance.

  - The objects import each other as siblings in `cas/objects/`, so the M3 import
    resolver links them with no path escaping the CAS sandbox. A case `import`s
    `objects/<hash>.adj` and the whole grounded graph comes with it.

  - Per object it writes a MANIFEST (`cas/objects/<hash>.json`): the kind, the
    dependency hashes, and - via the adversarial WRITE GATE - each clause's
    grounding status, declared vs accepted trust tier, and accept/flag verdict.

The write gate (deterministic here; it consumes the spider's adversarial
verification in `grounding/grounding-results.json`, optionally augmented by an
independent N-reader refute vote in `gate/votes.json`): a clause whose grounding
is `grounded` is ACCEPTED at its declared trust tier; a clause that is only
`direction_only` / `magnitude_leap` / ungrounded is FLAGGED and downgraded to
`inferred` - never deleted (dropping a prior would break the rulebook), so every
object stays runnable and the audit trail records trusted-vs-flagged.

Usage:  python3 cas_build.py            # build the CAS from lib/ + grounding/
        python3 cas_build.py --check    # build to a temp dir and diff (CI-safe)
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
LIB = ROOT / "lib"
GROUNDING = ROOT / "grounding" / "grounding-results.json"
GATE_VOTES = ROOT / "gate" / "votes.json"  # optional independent N-reader vote
CAS = ROOT / "cas"
OBJECTS = CAS / "objects"

IMPORT_RE = re.compile(r'^\s*import\s+"([^"]+)"\s*$', re.MULTILINE)
# A clause line: `prior <p> for <hyp>` or `contributes <lr> from <finding>(<val>) to <hyp>`
PRIOR_RE = re.compile(r"^\s*prior\s+([0-9.]+)\s+for\s+([a-z_]+)", re.MULTILINE)
CONTRIB_RE = re.compile(
    r"^\s*contributes\s+([0-9.]+)\s+from\s+([a-z_]+)\(([a-z_]+)\)\s+to\s+([a-z_]+)",
    re.MULTILINE,
)
TRUST_RE = re.compile(r"trust\s+(consensus|authoritative|empirical|inferred|unattributed)")

TRUST_ORDER = ["unattributed", "inferred", "empirical", "authoritative", "consensus"]


def sha(text: str) -> str:
    """16-hex-char content hash of a library's (rewritten) source."""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]


def lib_deps(src: str) -> list[str]:
    """The `import "X.adj"` targets of a library, in source order."""
    return IMPORT_RE.findall(src)


def topo_order(libs: dict[str, str]) -> list[str]:
    """Dependency order (leaves first) over the lib/*.adj graph. Acyclic by
    construction (the M3 resolver also rejects cycles); we assert it here."""
    order: list[str] = []
    seen: set[str] = set()
    visiting: set[str] = set()

    def visit(name: str) -> None:
        if name in seen:
            return
        if name in visiting:
            raise ValueError(f"import cycle through {name}")
        visiting.add(name)
        for dep in lib_deps(libs[name]):
            if dep not in libs:
                raise ValueError(f"{name} imports unknown library {dep}")
            visit(dep)
        visiting.discard(name)
        seen.add(name)
        order.append(name)

    for name in sorted(libs):
        visit(name)
    return order


def grounding_status_by_clause() -> dict[str, dict]:
    """Map a clause key -> its spider grounding record. Clause key for a prior is
    `prior_<hyp-arm>`; for a contribution it is `<finding>_<value>` matched to the
    grounding id (the spider ids are `<finding>_<value>` or `prior_<arm>`)."""
    if not GROUNDING.exists():
        return {}
    data = json.loads(GROUNDING.read_text())
    out: dict[str, dict] = {}
    for rec in data.get("records", []):
        out[rec["id"]] = rec
    return out


def clause_grounding_id(kind: str, finding: str | None, value: str | None, hyp: str) -> str:
    """Reconstruct the spider grounding id for a clause."""
    if kind == "prior":
        # prior_bacterial / prior_viral
        arm = "bacterial" if "bacterial" in hyp else "viral"
        return f"prior_{arm}"
    return f"{finding}_{value}"


def lr_matches(rulebook_lr: float, computed_lr: float | None) -> bool:
    """Does the rulebook's LR equal what the source's numbers entail? Accept a
    small tolerance (rounding / recalibration). This is the crux of the gate: the
    spider's `spider_status` was computed against the *authored* LR, but the
    rulebook was then RECALIBRATED toward the grounded `computed_lr`; the gate
    must judge the rulebook's ACTUAL value, not the pre-recalibration verdict."""
    if computed_lr is None:
        return False
    return abs(rulebook_lr - computed_lr) <= 0.05 * max(abs(rulebook_lr), abs(computed_lr)) + 0.01


def gate_clause(declared_trust: str, rulebook_lr: float, rec: dict | None,
                vote: dict | None) -> tuple[str, str, str]:
    """The adversarial write-gate decision for one clause.

    Returns (verdict, accepted_trust, reason). ACCEPT keeps the declared tier;
    FLAG downgrades to `inferred` (never deletes - every clause stays runnable).
    A clause is ACCEPTED iff:
      * the spider produced a grounding record whose quote survived independent
        re-extraction (`re_extraction_stable`), AND
      * the rulebook's LR matches the magnitude the source's numbers entail
        (`computed_lr`) - i.e. the rulebook was calibrated to the evidence, AND
      * no independent N-reader vote majority-refuted it (when such a vote exists).
    """
    if rec is None:
        return "FLAG", "inferred", "no grounding record"
    ver = rec.get("verification") or {}
    g = rec.get("grounded") or {}
    stable = bool(ver.get("re_extraction_stable"))
    matched = lr_matches(rulebook_lr, g.get("computed_lr"))
    vote_refuted = vote is not None and vote.get("majority") == "REFUTE"
    if stable and matched and not vote_refuted:
        return "ACCEPT", declared_trust, "re-extraction-stable; rulebook LR matches source"
    if not stable:
        reason = "quote did not survive independent re-extraction"
    elif not matched:
        reason = f"rulebook LR {rulebook_lr} != source-entailed {g.get('computed_lr')}"
    else:
        reason = "independent readers majority-refuted"
    return "FLAG", "inferred", reason


def parse_clauses(src: str) -> list[dict]:
    """Extract the gated clauses of a library (priors + contributions) with their
    declared trust tiers. Trust is read from the block following each clause head
    (the next `trust <tier>` on or after the clause line)."""
    clauses: list[dict] = []
    # Walk line ranges between clause heads so each clause picks up its own trust.
    heads: list[tuple[int, dict]] = []
    for m in PRIOR_RE.finditer(src):
        heads.append((m.start(), {"kind": "prior", "p": float(m.group(1)), "hyp": m.group(2),
                                  "finding": None, "value": None}))
    for m in CONTRIB_RE.finditer(src):
        heads.append((m.start(), {"kind": "contributes", "lr": float(m.group(1)),
                                  "finding": m.group(2), "value": m.group(3), "hyp": m.group(4)}))
    heads.sort(key=lambda h: h[0])
    for i, (pos, c) in enumerate(heads):
        end = heads[i + 1][0] if i + 1 < len(heads) else len(src)
        tm = TRUST_RE.search(src, pos, end)
        c["declared_trust"] = tm.group(1) if tm else "unattributed"
        if c["kind"] == "prior":
            c["key"] = f"{c['hyp']}::prior"
        else:
            c["key"] = f"{c['hyp']}::{c['finding']}({c['value']})"
        clauses.append(c)
    return clauses


def build(check: bool = False) -> int:
    libs = {p.name: p.read_text() for p in sorted(LIB.glob("*.adj"))}
    if not libs:
        print("cas_build: no lib/*.adj found", file=sys.stderr)
        return 2

    grounding = grounding_status_by_clause()
    votes = {}
    if GATE_VOTES.exists():
        votes = {v["id"]: v for v in json.loads(GATE_VOTES.read_text()).get("votes", [])}

    order = topo_order(libs)
    name_to_hash: dict[str, str] = {}
    objects: dict[str, dict] = {}  # hash -> {adj, manifest}

    for name in order:
        src = libs[name]
        # Rewrite imports to dependency hashes (deps already processed: topo order).
        rewritten = IMPORT_RE.sub(lambda m: f'import "{name_to_hash[m.group(1)]}.adj"', src)
        h = sha(rewritten)
        name_to_hash[name] = h

        kind = "dictionary" if "dictionary " in src else "rulebook"
        dep_hashes = [name_to_hash[d] for d in lib_deps(src)]

        gated_clauses = []
        for c in parse_clauses(src):
            gid = clause_grounding_id(c["kind"], c.get("finding"), c.get("value"), c["hyp"])
            rec = grounding.get(gid)
            vote = votes.get(gid)
            rulebook_lr = c.get("lr", c.get("p"))
            verdict, accepted_trust, reason = gate_clause(c["declared_trust"], rulebook_lr, rec, vote)
            gated_clauses.append({
                "key": c["key"],
                "lr": c.get("lr", c.get("p")),
                "declared_trust": c["declared_trust"],
                "accepted_trust": accepted_trust,
                "verdict": verdict,
                "grounding_id": gid,
                "spider_status": (rec or {}).get("spider_status", "none"),
                "reason": reason,
            })

        n_flag = sum(1 for c in gated_clauses if c["verdict"] == "FLAG")
        manifest = {
            "hash": h,
            "kind": kind,
            "source_name": name,
            "imports": dep_hashes,
            "clauses": gated_clauses,
            "gate": {
                "n_clauses": len(gated_clauses),
                "accepted": len(gated_clauses) - n_flag,
                "flagged_downgraded_to_inferred": [c["key"] for c in gated_clauses if c["verdict"] == "FLAG"],
            },
        }
        objects[h] = {"adj": rewritten, "manifest": manifest}

    registry = {
        "_doc": "MYCIN-2026 CAS registry. Each object is an importable adj-lang "
                "library, content-addressed by sha256[:16] of its source AFTER its "
                "imports are rewritten to dependency hashes (Merkle-style). A case "
                "imports objects/<hash>.adj to pull in the whole grounded graph.",
        "domain": "bacterial_vs_viral_meningitis",
        "names": {name: name_to_hash[name] for name in order},
        "root": name_to_hash.get("meningitis.adj"),
        "objects": {h: {"kind": o["manifest"]["kind"], "source_name": o["manifest"]["source_name"],
                        "imports": o["manifest"]["imports"],
                        "flagged": o["manifest"]["gate"]["flagged_downgraded_to_inferred"]}
                    for h, o in objects.items()},
    }

    if check:
        # CI mode: verify the on-disk CAS matches a fresh build.
        ok = True
        for h, o in objects.items():
            adj_path = OBJECTS / f"{h}.adj"
            man_path = OBJECTS / f"{h}.json"
            if not adj_path.exists() or adj_path.read_text() != o["adj"]:
                print(f"cas_build --check: {h}.adj out of date", file=sys.stderr)
                ok = False
            if not man_path.exists() or json.loads(man_path.read_text()) != o["manifest"]:
                print(f"cas_build --check: {h}.json out of date", file=sys.stderr)
                ok = False
        reg_path = CAS / "registry.json"
        if not reg_path.exists() or json.loads(reg_path.read_text()) != registry:
            print("cas_build --check: registry.json out of date", file=sys.stderr)
            ok = False
        if ok:
            print("cas_build --check: CAS is up to date.")
            return 0
        return 1

    OBJECTS.mkdir(parents=True, exist_ok=True)
    for h, o in objects.items():
        (OBJECTS / f"{h}.adj").write_text(o["adj"])
        (OBJECTS / f"{h}.json").write_text(json.dumps(o["manifest"], indent=2) + "\n")
    (CAS / "registry.json").write_text(json.dumps(registry, indent=2) + "\n")

    print(f"cas_build: wrote {len(objects)} content-addressed libraries to {OBJECTS}")
    for name in order:
        h = name_to_hash[name]
        flagged = objects[h]["manifest"]["gate"]["flagged_downgraded_to_inferred"]
        print(f"  {name:24s} -> {h}  ({objects[h]['manifest']['kind']}"
              + (f"; FLAGGED {flagged}" if flagged else "") + ")")
    print(f"  root (composed rulebook): objects/{registry['root']}.adj")
    return 0


if __name__ == "__main__":
    sys.exit(build(check="--check" in sys.argv[1:]))
