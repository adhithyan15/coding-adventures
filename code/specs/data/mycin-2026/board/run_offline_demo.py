#!/usr/bin/env python3
"""run_offline_demo.py — drive the offline board with a REAL local model (MYCIN-2026).

This is the live demonstration behind board_offline.py: it loads an actual on-device
MLX model, decomposes every prose board stem into an ADJ recall query, answers each
through the native adj-lang engine, and records a per-item transcript — with the
model decode AND the engine answer both executed INSIDE the network-egress guard, so
the run proves end-to-end that a small LOCAL model + a CPU reasoner pass the board
with zero online calls.

It is a demo/eval driver (not a unit test): it needs mlx_lm and a cached model, so it
is run by hand to produce board/offline-demo-transcript-<tag>.json. The deterministic,
dependency-free contract lives in board_offline.py + test_board_offline.py.

Usage (run with a python that has mlx_lm; HF_HUB_OFFLINE=1 forces cache-only load):
    HF_HUB_OFFLINE=1 python run_offline_demo.py mlx-community/gemma-3-4b-it-bf16 gemma3-4b
    HF_HUB_OFFLINE=1 python run_offline_demo.py Qwen/Qwen2.5-0.5B-Instruct qwen2.5-0.5b
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import board_eval as be  # noqa: E402
import decompose_query as dq  # noqa: E402
from offline_guard import no_network  # noqa: E402

ITEMS = HERE / "free_text_board.json"


def run(model_path: str, tag: str) -> dict:
    items = json.loads(ITEMS.read_text())["items"]
    vocab = dq.build_vocab()
    gen = dq.local_gen(model_path)  # load happens here (cache-only under HF_HUB_OFFLINE=1)

    records = []
    guard = no_network()
    with guard:  # BOTH the local decode and the engine answer run with the net blocked
        for it in items:
            raw = gen(dq.build_query_prompt(it["stem"], vocab))
            q_parsed = dq.parse_query(raw)  # pre-gate parse (for transcript + decompose_ok)
            decompose_ok = q_parsed is not None and all(
                q_parsed.get(k) == it["query"][k] for k in ("relation", "subject", "var")
            )
            # Faithfulness gate: the chosen subject must be attested by the stem's bytes,
            # else the query is rejected → the engine abstains instead of answering the
            # wrong question (byte-provenance applied to the decomposition).
            faithful = q_parsed is not None and dq.attested_in_stem(q_parsed["subject"], it["stem"])
            q = q_parsed if faithful else None
            faithful_rejected = q_parsed is not None and not faithful
            ans = be.resolve_recall([{"id": it["id"], **q}]).get(it["id"]) if q else None
            engine_answer = None if (ans is None or ans["abstained"]) else ans["answer"]
            gold = it["gold"]
            if gold == "ABSTAIN":
                outcome = "abstained" if engine_answer is None else "wrong"
            elif engine_answer is None:
                outcome = "abstained"
            else:
                outcome = "correct" if engine_answer == gold else "wrong"
            records.append({
                "id": it["id"], "domain": it["domain"], "stem": it["stem"],
                "model_raw": (raw or "")[:200], "model_query": q_parsed, "gold_query": it["query"],
                "decompose_ok": decompose_ok, "faithful_rejected": faithful_rejected,
                "engine_answer": engine_answer, "gold": gold, "outcome": outcome,
                "trust": (ans or {}).get("trust") if engine_answer else None,
            })

    n = len(records)
    correct = sum(1 for r in records if r["outcome"] == "correct")
    wrong = sum(1 for r in records if r["outcome"] == "wrong")
    abstained = sum(1 for r in records if r["outcome"] == "abstained")
    dec_ok = sum(1 for r in records if r["decompose_ok"])
    faithful_rejected = sum(1 for r in records if r["faithful_rejected"])
    summary = {
        "model": model_path, "tag": tag, "total": n,
        "correct": correct, "abstained": abstained, "wrong": wrong,
        "defensibility": round((correct + abstained) / n, 4) if n else 0.0,
        "decompose_accuracy": round(dec_ok / n, 4) if n else 0.0,
        "faithful_rejected": faithful_rejected,
        "online_calls": len(guard.attempts),
    }
    out = {"summary": summary, "records": records}
    dest = HERE / f"offline-demo-transcript-{tag}.json"
    dest.write_text(json.dumps(out, indent=2) + "\n")

    print(f"\n[{tag}] {model_path}")
    print(f"  end-to-end: correct {correct} · abstained {abstained} · wrong {wrong} (of {n})")
    print(f"  decompose accuracy {summary['decompose_accuracy']:.0%}  ·  "
          f"faithfulness-rejected {faithful_rejected}  ·  "
          f"defensibility {summary['defensibility']:.0%}  ·  online calls {summary['online_calls']}")
    print(f"  → {dest.name}")
    return out


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("usage: run_offline_demo.py <model_path> <tag>", file=sys.stderr)
        sys.exit(2)
    run(sys.argv[1], sys.argv[2])
