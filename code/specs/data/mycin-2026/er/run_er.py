#!/usr/bin/env python3
"""run_er.py - the ER spine: someone walks in, speaks, gets triaged. Fully local.

MYCIN-2026 C3. The "someone comes into the ER" demo the project is aimed at, end
to end and entirely on-device:

    voice / typed prose
        |  transcribe        (mlx-whisper, on-device — audio never leaves)
        v  transcript
        |  decompose_text    (the ONE local model call — C2 backend selection)
        v  typed findings
        |  ir_to_adj -> decide   (CPU engine, 0 answer-time model calls, byte-cited)
        v  differential + what-to-check-next
        |  triage            (grounded ESI acuity + immediate actions, 0 model calls)
        v  ACUITY + IMMEDIATE-ACTION CHECKLIST + audit trail

Decision SUPPORT: every line is grounded + overridable; the triage nurse /
physician makes the call. Nothing is sent off the machine.

Usage:
  python3 run_er.py "72M brought in febrile, neck stiff, then a seizure"
  python3 run_er.py path/to/recording.wav     # if mlx-whisper is installed
"""

from __future__ import annotations

import sys
from pathlib import Path

MYCIN = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(MYCIN / "warm"))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import decide as decide_mod  # noqa: E402
import decomposer as dc  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402
import transcribe as tr  # noqa: E402
import triage as triage_mod  # noqa: E402


def run(source: str) -> int:
    cli = decide_mod.find_cli()
    if cli is None:
        print("run_er: adj-lang-cli not built", file=sys.stderr)
        return 3
    try:
        backend, gen = dc.select_backend()
    except RuntimeError as e:
        print(f"run_er: {e}", file=sys.stderr)
        return 1

    transcript = tr.transcribe(source)
    bar = "=" * 78
    print(bar)
    print(f"ER INTAKE: {transcript}")
    print(f"[transcribe: {'mlx-whisper' if tr.is_audio_path(source) else 'typed transcript'}"
          f"  |  decompose: {backend}]  — on-device; nothing leaves the machine")
    print(bar)

    # decompose (1 local model call) -> findings
    domains = ir_mod.load_domains()
    ir = dc.decompose_text(transcript, gen=gen)
    observe_adj, kept, dropped = ir_mod.ir_to_adj(ir, domains)
    print(f"\n[1] FINDINGS: {', '.join(kept) or '(none mapped)'}")

    # differential (0 model calls)
    leader = decision_type = None
    if kept:
        res = decide_mod.decide("er", observe_adj, cli)
        leader, decision_type = res["leader"], res["decision"].get("type")
        print("\n[2] DIFFERENTIAL (0 answer-time model calls):")
        for hyp, p in sorted(res["posteriors"].items(), key=lambda kv: -kv[1]):
            lead = "  <- leading" if hyp == leader else ""
            print(f"    {hyp:24s} P = {p:.4f}{lead}")
    else:
        print("\n[2] No dictionary findings — the differential abstains.")

    # triage (0 model calls, grounded rules)
    t = triage_mod.triage(leader, decision_type, kept)
    print(f"\n[3] TRIAGE — ESI acuity {t['acuity']} ({t['label']})"
          + (f", target {t['time_target_min']} min" if t.get("time_target_min") else "")
          + f"   [rule: {t['rule']}]")
    print("    IMMEDIATE ACTIONS:")
    for a in t["immediate_actions"]:
        print(f"      - {a}")
    print(f"    [{t['source']}]")

    print("\n" + bar)
    print("PHYSICIAN / TRIAGE-NURSE REVIEW — grounded + overridable; you make the call.")
    print("answer-time model calls: 0   |   audio/data left the machine: none")
    return 0


def main(argv: list[str]) -> int:
    source = " ".join(a for a in argv if not a.startswith("--"))
    if not source:
        print('usage: run_er.py "<prose>" | <recording.wav>', file=sys.stderr)
        return 2
    return run(source)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
