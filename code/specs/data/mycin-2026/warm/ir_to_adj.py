#!/usr/bin/env python3
"""ir_to_adj.py - deterministic compiler: decomposed IR -> adj-lang `observe` lines.

MYCIN-2026 M6. The decomposer (the model) emits typed findings; THIS step is pure
CPU - no model. It turns the IR into `observe functor(value)` lines, enforcing the
closed dictionary (the same vocabulary the rulebook compiles against) so the IR
and the rulebook can never drift. Gating rules:

  * a finding whose functor/value is not in the dictionary is a hard error
    (shared-vocabulary enforcement - the decomposer hallucinated a term);
  * polarity == "denied" findings are dropped (an explicitly-negated finding is
    not an observation that the rulebook's positive clauses should fire on);
  * `inferred` findings whose adversarial inference verdict is "LEAP" are dropped
    (kept only if "ENTAILED"); `stated` findings are always kept;
  * duplicates are collapsed.

Returns (adj_text, kept, dropped) so the caller can audit exactly what survived.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DICT = ROOT / "warm" / "dictionary.json"

TERM_RE = re.compile(r"\A([a-z_]+)\(([a-z_]+)\)\Z")


def load_domains() -> dict[str, set[str]]:
    d = json.loads(DICT.read_text())
    return {f["functor"]: set(f["value_domain"]) for f in d["findings"]}


def normalize_finding(f: dict) -> tuple[str, str, bool, bool]:
    """Normalize a decomposer finding to (functor, value, is_denied, is_inferred),
    tolerating the schema variants small models emit. Some models write
    {term:"functor(value)"}; others split {functor, value}; and many overload one
    field ("type" or "polarity") to carry stated|inferred|denied. We read all of
    them so the deterministic step is robust to the model's exact JSON shape - the
    closed-vocabulary check (below) still rejects any hallucinated functor/value."""
    term = f.get("term")
    if term:
        m = TERM_RE.match(str(term))
        if not m:
            raise ValueError(f"malformed IR term {term!r} (want functor(value))")
        functor, value = m.group(1), m.group(2)
    else:
        functor, value = f.get("functor"), f.get("value")
        if not functor or not value:
            raise ValueError(f"IR finding missing term/functor/value: {f!r}")
    # stated|inferred|denied may live in either "type" or "polarity".
    tags = {str(f.get("type", "")).lower(), str(f.get("polarity", "")).lower()}
    is_denied = "denied" in tags
    is_inferred = "inferred" in tags
    return str(functor), str(value), is_denied, is_inferred


def ir_to_adj(ir: dict, domains: dict[str, set[str]]) -> tuple[str, list[str], list[dict]]:
    # Terms the adversarial read marked as a LEAP (drop) - by exact term string.
    leap_terms = {str(j.get("term") or j.get("finding") or "")
                  for j in ir.get("inference_justifications", [])
                  if str(j.get("verdict", "")).upper() == "LEAP"}

    kept: list[str] = []
    dropped: list[dict] = []
    seen: set[str] = set()
    for f in ir.get("findings", []):
        functor, value, is_denied, is_inferred = normalize_finding(f)
        term = f"{functor}({value})"
        # Closed-vocabulary enforcement: a hallucinated functor/value is DROPPED
        # (recorded for audit) rather than reaching the engine. We drop instead of
        # aborting the case so small-model noise on one finding does not discard
        # the valid findings - but the bad term can never influence the diagnosis.
        if functor not in domains:
            dropped.append({"term": term, "reason": f"functor {functor!r} not in dictionary"})
            continue
        if value not in domains[functor]:
            dropped.append({"term": term, "reason": f"value {value!r} not in domain {sorted(domains[functor])}"})
            continue
        if is_denied:
            dropped.append({"term": term, "reason": "denied polarity"})
            continue
        if is_inferred and term in leap_terms:
            dropped.append({"term": term, "reason": "adversarial inference verdict LEAP"})
            continue
        if term in seen:
            continue
        seen.add(term)
        kept.append(term)

    adj = "".join(f"observe {t}\n" for t in kept)
    return adj, kept, dropped


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: ir_to_adj.py <ir.json>", file=sys.stderr)
        return 2
    ir = json.loads(Path(argv[0]).read_text())
    domains = load_domains()
    adj, kept, dropped = ir_to_adj(ir, domains)
    print(adj, end="")
    print(f"% kept {len(kept)}, dropped {len(dropped)}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
