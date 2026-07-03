#!/usr/bin/env python3
"""run_live.py - messy human input -> diagnosis, FULLY LOCAL. The C2 payoff.

One raw clinical string in; a byte-cited differential out. The ONLY model call is
the on-device decompose (decomposer.py picks the local specialist or local
Ollama); everything after is the CPU engine over the grounded CAS rulebook at
0 ANSWER-TIME model calls. So the patient's words never leave the machine - the
privacy / HIPAA story, demonstrated end to end:

    "72M, fever, stiff neck, neutrophilic CSF, low glucose"
        |  decomposer.decompose_text   (1 on-device model call)
        v  typed findings in the closed dictionary
        |  ir_to_adj -> decide          (0 model calls, CPU, byte-cited)
        v  differential + the audit trail the physician reviews

This is decision SUPPORT: every line is grounded + overridable; the physician
makes the call. Run it on real prose; it never sends anything off-device.

Usage:  python3 run_live.py "<clinical prose>"
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "warm"))
import decide as decide_mod  # noqa: E402
import decomposer as dc  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402


def run(prose: str) -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("run_live: adj-lang-cli not built", file=sys.stderr)
        return 3
    try:
        backend, gen = dc.select_backend()
    except RuntimeError as e:
        print(f"run_live: {e}", file=sys.stderr)
        return 1

    print("=" * 74)
    print(f"INPUT (messy human prose): {prose}")
    print(f"[decompose backend: {backend}] - the ONLY model call, on-device")
    print("=" * 74)

    # [1] the single on-device model call: prose -> typed IR
    domains = ir_mod.load_domains()
    ir = dc.decompose_text(prose, gen=gen)
    observe_adj, kept, dropped = ir_mod.ir_to_adj(ir, domains)
    print(f"\n[1] DECOMPOSITION  findings: {', '.join(kept) or '(none mapped)'}")
    if dropped:
        print(f"    dropped at the vocabulary gate: {[d['term'] for d in dropped]}")
    if not kept:
        print("\n    No dictionary findings extracted -> the engine abstains "
              "(no fabricated diagnosis). Re-phrase or add detail.")
        return 0

    # [2] the differential - 0 answer-time model calls, byte-cited
    res = decide_mod.decide("live", observe_adj, cli)
    print("\n[2] DIFFERENTIAL  (0 answer-time model calls - CPU over the grounded rulebook)")
    for hyp, p in sorted(res["posteriors"].items(), key=lambda kv: -kv[1]):
        lead = "  <- leading" if hyp == res["leader"] else ""
        print(f"    {hyp:24s} P = {p:.4f}{lead}")
    dec = res["decision"].get("type")
    print(f"    decision: {dec}  | evidence for leader: {res['n_evidence_for_leader']}")

    print("\n" + "=" * 74)
    print("PHYSICIAN REVIEW - every line is grounded + overridable; you make the call.")
    print("answer-time model calls: 0   |   data left the machine: none")
    return 0


def main(argv: list[str]) -> int:
    prose = " ".join(a for a in argv if not a.startswith("--"))
    if not prose:
        print('usage: run_live.py "<clinical prose>"', file=sys.stderr)
        return 2
    return run(prose)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
