#!/usr/bin/env python3
"""eval_argument_decompose.py — score an ARGUMENT decomposer against a held-out eval set.

The argument-shape counterpart to eval_decompose.py. `argument_decompose_score.py` scores one
predicted argument's fidelity; this ties it to a small, CURATED, held-out benchmark
(`argument_decompose_eval.jsonl`) of prose paragraphs paired with gold arguments across DISTINCT
domains (law / economics / chemistry — disjoint from the AD-2 seed), plus a runner that scores a
model's predictions and reports the aggregate.

The eval set is HAND-CURATED and every gold passes the three-part correctness gate (it compiles,
derives its thesis, and byte-anchors every citation — validated at authoring time via
gen_argument_data). `test_eval_argument_decompose.py` pins that the gold is self-consistent.

Three ways to run:
  * `--self-check` (offline, no model): score each GOLD as its own prediction — every ratio must
    be a perfect 1.0 with zero vetoes; with the binaries built, each gold must also DERIVE its
    thesis. The CI sanity check that the set + scorer compose.
  * `--pred predictions.jsonl`: score a model's predictions. Each line is a gold-object plus `id`
    matching an eval id; a missing id scores as an empty argument (penalized, never skipped).
  * `--model <path>` (optional, needs MLX): run the model as the decomposer over each note, parse
    its emitted argument, and score. Skips gracefully when MLX is unavailable.

Usage:
  python3 eval_argument_decompose.py --self-check
  python3 eval_argument_decompose.py --pred my_predictions.jsonl
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import argument_decompose_score as ads  # noqa: E402

EVAL_SET = HERE / "argument_decompose_eval.jsonl"
_EMPTY = {"premises": [], "inferences": [], "discard": [], "thesis": ""}


def load_eval(path: Path = EVAL_SET) -> list[dict]:
    """Load the curated held-out eval records (one JSON object per line)."""
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def score_predictions(records: list[dict], predictions: dict[str, dict],
                      *, run_gate: bool = True) -> tuple[list[dict], dict]:
    """Score each eval record's prediction (by id) against its gold, returning the per-example rows
    (id + domain + metrics) and the aggregate. A record with no prediction is scored as an empty
    argument — so silent non-coverage is penalized, not skipped. The prediction inherits the gold's
    thesis when it omits one (a decomposer emits premises/inferences; the thesis query is fixed)."""
    rows, scores = [], []
    for rec in records:
        pred = dict(predictions.get(rec["id"], _EMPTY))
        pred.setdefault("thesis", rec["gold"].get("thesis", ""))
        s = ads.score(pred, rec["gold"], rec["note"], run_gate=run_gate)
        rows.append({"id": rec["id"], "domain": rec.get("domain", "?"), **s})
        scores.append(s)
    return rows, ads.aggregate(scores)


def self_check(records: list[dict], *, run_gate: bool) -> int:
    """Score each gold as its own prediction; every ratio must be 1.0 and every veto 0 (and, with
    the gate, every thesis must derive). Returns 0 on a perfect set, 1 otherwise."""
    predictions = {r["id"]: r["gold"] for r in records}
    rows, agg = score_predictions(records, predictions, run_gate=run_gate)
    bad = 0
    for r in rows:
        ratios_ok = all(r[m] == 1.0 for m in (
            "premise_precision", "premise_recall", "premise_f1",
            "inference_precision", "inference_recall", "inference_f1", "span_faithfulness"))
        vetoes_ok = r["near_miss_violations"] == 0 and r["fabrication"] == 0
        gate_ok = r["thesis_derivation"] in (None, 1)
        ok = ratios_ok and vetoes_ok and gate_ok
        bad += 0 if ok else 1
        print(f"  [{'ok' if ok else 'FAIL'}] {r['id']:22s} {r['domain']:11s} "
              f"prem_f1={r['premise_f1']:.2f} inf_f1={r['inference_f1']:.2f} "
              f"span={r['span_faithfulness']:.2f} near_miss={r['near_miss_violations']} "
              f"fab={r['fabrication']} derive={r['thesis_derivation']}")
    if bad:
        print(f"\nself-check: {bad} gold example(s) did NOT self-score perfectly", file=sys.stderr)
        return 1
    print(f"\nself-check: all {agg.get('n', 0)} gold arguments self-score perfectly ✓")
    return 0


def parse_argument(raw: str) -> dict:
    """Extract the first JSON object from a model's raw decode (it may wrap JSON in prose/fences).
    Returns an empty argument if nothing parses — scored as non-coverage, never crashes."""
    i, j = raw.find("{"), raw.rfind("}")
    if i == -1 or j <= i:
        return dict(_EMPTY)
    try:
        obj = json.loads(raw[i:j + 1])
        return obj if isinstance(obj, dict) else dict(_EMPTY)
    except (ValueError, TypeError):
        return dict(_EMPTY)


def build_prompt(rec: dict) -> str:
    """The decompose prompt: ask the model to emit the gold-object argument schema (§3.2) from the
    paragraph, citing VERBATIM spans. Kept here (not a heavy import) so the offline path is light."""
    return (
        "Decompose the following paragraph into an ARGUMENT as JSON with keys "
        '"premises" (each {"name","kind","term","span","type"}), "inferences" '
        '(each {"name","connective","conclusion","from","span","type"}), and "discard" '
        '(each {"span","reason"}). Every "span" MUST be a verbatim substring of the paragraph. '
        "Do not invent facts; set aside irrelevant sentences as discards.\n\nPARAGRAPH:\n"
        + rec["note"]
    )


def predict_with_model(records: list[dict], gen) -> dict[str, dict]:
    """Run a text-generation `gen(prompt) -> str` (an MLX model, or a test stub) over each eval
    note and parse its output into a predicted argument keyed by id. `gen` is the only dependency,
    so the model is injectable and this is unit-testable without MLX."""
    return {rec["id"]: parse_argument(gen(build_prompt(rec))) for rec in records}


def _mlx_gen(model_path: str, adapter: str | None):
    """Lazy MLX text generator (mirrors eval_decompose._mlx_gen) — imported only for a --model run,
    so the offline path pulls no heavy deps."""
    from mlx_lm import generate, load  # noqa: PLC0415
    from mlx_lm.sample_utils import make_sampler  # noqa: PLC0415
    model, tok = load(model_path, adapter_path=adapter)
    sampler = make_sampler(temp=0.0)

    def gen(prompt: str) -> str:
        text = tok.apply_chat_template([{"role": "user", "content": prompt}],
                                       add_generation_prompt=True)
        out = generate(model, tok, prompt=text, max_tokens=512, sampler=sampler, verbose=False)
        for stop in ("<end_of_turn>", "<eos>", "<pad>", "<start_of_turn>"):
            out = out.split(stop)[0]
        return out.strip()
    return gen


def _print_scored(records: list[dict], predictions: dict[str, dict]) -> int:
    import gen_argument_data as gad  # noqa: PLC0415
    run_gate = gad.CLI.exists() and gad.VERIFY.exists()
    rows, agg = score_predictions(records, predictions, run_gate=run_gate)
    for r in rows:
        print(f"  {r['id']:22s} {r['domain']:11s} prem_f1={r['premise_f1']:.2f} "
              f"inf_f1={r['inference_f1']:.2f} span={r['span_faithfulness']:.2f} "
              f"near_miss={r['near_miss_violations']} fab={r['fabrication']} "
              f"derive={r['thesis_derivation']}")
    td = agg.get("thesis_derivation")
    print(f"\naggregate over {agg.get('n', 0)}: premise_f1={agg.get('premise_f1', 0):.3f} "
          f"inference_f1={agg.get('inference_f1', 0):.3f} "
          f"span_faithfulness={agg.get('span_faithfulness', 0):.3f} "
          f"near_miss_violations={agg.get('near_miss_violations', 0)} "
          f"fabrication={agg.get('fabrication', 0)} "
          f"thesis_derivation={'n/a' if td is None else f'{td:.3f}'}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-check", action="store_true",
                    help="score the gold as the prediction (must be perfect); offline sanity check")
    ap.add_argument("--pred", type=Path, help="JSONL of model predictions keyed by eval id")
    ap.add_argument("--model", help="run this MLX model as the decomposer over the eval notes")
    ap.add_argument("--adapter", default=None, help="optional LoRA adapter path for --model")
    args = ap.parse_args()

    records = load_eval()
    if args.self_check:
        import gen_argument_data as gad  # noqa: PLC0415
        return self_check(records, run_gate=gad.CLI.exists() and gad.VERIFY.exists())
    if args.model:
        predictions = predict_with_model(records, _mlx_gen(args.model, args.adapter))
    elif args.pred:
        predictions = {p["id"]: p for p in
                       (json.loads(line) for line in args.pred.read_text().splitlines() if line.strip())}
    else:
        print("eval_argument_decompose: pass --self-check, --model <path>, or --pred <file>",
              file=sys.stderr)
        return 2
    return _print_scored(records, predictions)


if __name__ == "__main__":
    sys.exit(main())
