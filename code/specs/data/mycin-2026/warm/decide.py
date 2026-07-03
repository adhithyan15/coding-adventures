#!/usr/bin/env python3
"""decide.py - link a decomposed case to the CAS rulebook and decide. 0 model calls.

MYCIN-2026 M6. The warm path's final, CPU-bound step: take the IR (from the
model's decompose-only pass) -> `observe` lines (ir_to_adj) -> a case program that
`import`s the content-addressed rulebook object from the CAS -> run `adj-lang-cli`
-> the differential diagnosis + proof DAG. The model is NOT in this loop:
`answer_time_model_calls == 0` by construction.

  case program (written to cas/<id>.linked.adj so `import "objects/<hash>.adj"`
  stays inside the CAS sandbox):

      import "objects/<root-hash>.adj"   % pulls vocab + arms + rulebook + ? queries
      observe csf_gram_stain(positive)
      ...

Evidence-sufficiency guard (abstain, do not fabricate): if the leading
hypothesis's proof fired ZERO contribution/interaction steps - i.e. the verdict
rests on the prior alone - the decision is overridden to `insufficient_evidence`
rather than diagnosing on the base rate.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPO = ROOT.parents[3]
CAS = ROOT / "cas"
HASH_RE = re.compile(r"\A[0-9a-f]{16}\Z")
# case_id becomes a filename (cas/<case_id>.linked.adj). It can come from a
# model-generated IR, so it is semi-untrusted: restrict it to a safe charset and
# confirm the resolved path stays inside cas/ (no `../` arbitrary-write escape).
CASE_ID_RE = re.compile(r"\A[A-Za-z0-9_-]{1,64}\Z")


def find_cli() -> Path | None:
    p = shutil.which("adj-lang-cli")
    if p:
        return Path(p)
    for prof in ("debug", "release"):
        c = REPO / "code/packages/rust/target" / prof / "adj-lang-cli"
        if c.exists():
            return c
    return None


def root_hash() -> str:
    h = json.loads((CAS / "registry.json").read_text())["root"]
    assert h and HASH_RE.match(h), f"registry root not a valid hash: {h!r}"
    return h


def decide(case_id: str, observe_adj: str, cli: Path) -> dict:
    """Link + run. Returns the warm-path result row (0 answer-time model calls)."""
    if not CASE_ID_RE.match(case_id):
        raise ValueError(f"unsafe case_id {case_id!r} (must match {CASE_ID_RE.pattern})")
    linked = (CAS / f"{case_id}.linked.adj").resolve()
    if linked.parent != CAS.resolve():
        raise ValueError(f"case_id escapes the CAS dir: {case_id!r}")
    linked.write_text(f'import "objects/{root_hash()}.adj"\n{observe_adj}')
    try:
        r = subprocess.run([str(cli), str(linked)], capture_output=True, text=True)
        assert r.returncode == 0, f"adj-lang-cli exited {r.returncode}: {r.stderr}"
        out = json.loads(r.stdout)
    finally:
        linked.unlink(missing_ok=True)

    decision = out.get("decision", {})
    ranked = out.get("ranked", [])
    leader = decision.get("leader") or (ranked[0]["hypothesis"] if ranked else None)

    # Evidence sufficiency: count the contribution/interaction steps in the leader's proof.
    n_evidence = 0
    for r_ in ranked:
        if r_["hypothesis"] == leader:
            n_evidence = sum(1 for s in r_.get("proof", [])
                             if s.get("kind") in ("contribution", "interaction", "predicate"))
    if n_evidence == 0:
        decision = {"type": "insufficient_evidence",
                    "note": "leader rests on the prior alone; abstaining"}

    return {
        "case_id": case_id,
        "answer_time_model_calls": 0,
        "decision": decision,
        "leader": leader,
        "n_evidence_for_leader": n_evidence,
        "posteriors": {r_["hypothesis"]: r_["posterior"] for r_ in ranked},
    }


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: decide.py <case_id> <observe.adj-file>", file=sys.stderr)
        return 2
    cli = find_cli()
    if cli is None:
        print("decide: adj-lang-cli not built (cargo build -p adj-lang-cli)", file=sys.stderr)
        return 3
    observe_adj = Path(argv[1]).read_text()
    print(json.dumps(decide(argv[0], observe_adj, cli), indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
