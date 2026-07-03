#!/usr/bin/env python3
"""eval_specialist.py - score an MLX model (base, or base+LoRA) as the decomposer.

Runs an MLX-served model (Gemma/Qwen base, optionally with a trained LoRA adapter)
as the warm-path decomposer over the bench vignettes, then through the SAME
deterministic framework (ir_to_adj -> decide) and scores vs gold - identical to
bench_models.py, so base-vs-specialist is apples-to-apples. The model never
diagnoses; it only decomposes.

Usage:
  python3 eval_specialist.py --model <hf-or-path> [--adapter adapters/] [--cases ...]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

MYCIN = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(MYCIN / "warm"))
sys.path.insert(0, str(MYCIN / "bench"))
import bench_models as bench  # noqa: E402  (tolerant_findings)
import decide as decide_mod  # noqa: E402
import decompose as decompose_mod  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402

CASES = MYCIN / "cases" / "cases.json"
DICT = MYCIN / "warm" / "dictionary.json"


def mlx_generate_fn(model_path: str, adapter: str | None):
    from mlx_lm import generate, load
    from mlx_lm.sample_utils import make_sampler
    model, tok = load(model_path, adapter_path=adapter)
    sampler = make_sampler(temp=0.0)  # greedy / deterministic

    def gen(prompt: str) -> str:
        text = tok.apply_chat_template([{"role": "user", "content": prompt}],
                                       add_generation_prompt=True)
        out = generate(model, tok, prompt=text, max_tokens=512, sampler=sampler, verbose=False)
        # Trim Gemma turn-end / special tokens so the JSON parses cleanly.
        for stop in ("<end_of_turn>", "<eos>", "<pad>", "<start_of_turn>"):
            out = out.split(stop)[0]
        return out.strip()
    return gen


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--adapter", default=None)
    args = ap.parse_args()

    cli = decide_mod.find_cli()
    if cli is None:
        print("eval: adj-lang-cli not built", file=sys.stderr)
        return 3
    d = json.loads(DICT.read_text())
    cases = json.loads(CASES.read_text())["cases"]
    domains = ir_mod.load_domains()
    gen = mlx_generate_fn(args.model, args.adapter)

    label = f"{args.model}" + (f" + {args.adapter}" if args.adapter else " (base)")
    print(f"=== {label} ===")
    n_correct = n_wrong = n_abst = n_fail = 0
    for c in cases:
        raw = gen(decompose_mod.prompt_for(c["vignette"], d))
        ir = bench.tolerant_findings(decompose_mod.coerce_ir(c["id"], raw), domains)
        try:
            obs, kept, dropped = ir_mod.ir_to_adj(ir, domains)
        except Exception as e:  # noqa: BLE001
            print(f"  {c['id']:26s} ir_error {str(e)[:50]}")
            n_fail += 1
            continue
        if not kept:
            print(f"  {c['id']:26s} no_findings")
            n_fail += 1
            continue
        res = decide_mod.decide(c["id"], obs, cli)
        leader, dtype = res["leader"], res["decision"].get("type")
        if dtype == "insufficient_evidence":
            score = "abstained"
            n_abst += 1
        elif leader == c["gold"]:
            score = "correct"
            n_correct += 1
        else:
            score = "wrong"
            n_wrong += 1
        print(f"  {c['id']:26s} {score:11s} leader={leader} gold={c['gold']} "
              f"findings={len(kept)} dropped={len(dropped)}")
    print(f"\n  {n_correct}/{len(cases)} correct, {n_abst} abstained, {n_wrong} wrong, "
          f"{n_fail} failed  (answer-time model calls: 0)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
