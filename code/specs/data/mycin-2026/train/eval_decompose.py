#!/usr/bin/env python3
"""eval_decompose.py — score a decomposer against the held-out decompose-fidelity eval set.

`eval_specialist.py` scores the downstream DIAGNOSIS a model's decomposition produces.
`decompose_score.py` scores decompose FIDELITY (facts/spans/discards/near-misses) for one
example. This ties them together: a small, CURATED, held-out eval set (`decompose_eval.jsonl`)
of prose vignettes paired with gold typed IR — across both shapes (chart-facts + findings) and
the hard cases (near-miss family-history / absence / efficacy / hedge, and honest abstain) — plus
a runner that scores a model's predictions against it and reports the aggregate.

The eval set is HAND-CURATED (not teacher-generated) so it is a stable benchmark: edits are
deliberate, and `test_eval_decompose.py` pins that the gold is self-consistent (every span
verbatim in its note; every chart-fact COP-consumable; every near-miss a discard, not a fact).

Two ways to run:
  * `--self-check` (offline, no model): score the GOLD as if it were the prediction — must be a
    perfect 1.0 with zero violations. A CI sanity check that the set + the scorer compose; this is
    what the test exercises.
  * `--pred predictions.jsonl`: score a model's predictions. Each line `{"id", "<shape-field>",
    "discard"}` matching an eval id; the model run that produced them is the caller's step (e.g.
    pipe `eval_specialist`'s decomposer over the eval notes). Missing ids score as empty (abstain).

Usage:
  python3 eval_decompose.py --self-check
  python3 eval_decompose.py --pred my_model_predictions.jsonl
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import decompose_score as ds  # noqa: E402

EVAL_SET = HERE / "decompose_eval.jsonl"
SHAPES = {"chart_facts": ds.CHART_FACTS, "findings": ds.FINDINGS}


def load_eval(path: Path = EVAL_SET) -> list[dict]:
    """Load the curated held-out eval records (one JSON object per line)."""
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def score_predictions(records: list[dict], predictions: dict[str, dict]) -> tuple[list[dict], dict]:
    """Score each eval record's prediction (by id) against its gold, returning the per-example
    rows (id + shape + metrics) and the aggregate. A record with no prediction is scored as an
    empty IR (the model emitted nothing) — so silent non-coverage is penalized, not skipped."""
    rows, scores = [], []
    for rec in records:
        shape = SHAPES[rec["shape"]]
        items_field = shape[0]
        pred = predictions.get(rec["id"], {items_field: [], "discard": []})
        s = ds.score_decompose(pred, rec["gold"], rec["note"], shape)
        rows.append({"id": rec["id"], "shape": rec["shape"], **s})
        scores.append(s)
    return rows, ds.aggregate(scores)


def parse_ir(raw: str) -> dict:
    """Extract the first JSON object from a model's raw decode (it may wrap the JSON in prose or
    code fences). Returns {} if nothing parses — scored as an empty IR (penalized, never crashes)."""
    i, j = raw.find("{"), raw.rfind("}")
    if i == -1 or j <= i:
        return {}
    try:
        return json.loads(raw[i:j + 1])
    except (ValueError, TypeError):
        return {}


def build_prompt(rec: dict, dictionary: dict) -> str:
    """The shape-appropriate decompose prompt the model is asked — the SAME prompts the training
    data was generated under (gen_chart_data for chart-facts; warm/decompose for findings), so the
    eval matches training. Imported lazily so the offline path pulls no heavy modules."""
    if rec["shape"] == "chart_facts":
        import gen_chart_data  # noqa: PLC0415
        return gen_chart_data.prompt_for_chart(rec["note"])
    warm = str(HERE.parent / "warm")
    if warm not in sys.path:
        sys.path.insert(0, warm)
    import decompose  # noqa: PLC0415
    return decompose.prompt_for(rec["note"], dictionary)


def predict_with_model(records: list[dict], gen, dictionary: dict) -> dict[str, dict]:
    """Run a text-generation `gen(prompt) -> str` (an MLX model, or a test stub) over each eval
    note and parse its output into a predicted IR keyed by id. `gen` is the only dependency, so
    the model is fully injectable and this is unit-testable without MLX."""
    return {rec["id"]: parse_ir(gen(build_prompt(rec, dictionary))) for rec in records}


def _mlx_gen(model_path: str, adapter: str | None):
    """Lazy MLX text generator (mirrors eval_specialist.mlx_generate_fn) — imported only when a
    --model run is requested, so importing this module stays dependency-free for the offline path."""
    from mlx_lm import generate, load  # noqa: PLC0415
    from mlx_lm.sample_utils import make_sampler  # noqa: PLC0415
    model, tok = load(model_path, adapter_path=adapter)
    sampler = make_sampler(temp=0.0)  # greedy / deterministic

    def gen(prompt: str) -> str:
        text = tok.apply_chat_template([{"role": "user", "content": prompt}],
                                       add_generation_prompt=True)
        out = generate(model, tok, prompt=text, max_tokens=512, sampler=sampler, verbose=False)
        for stop in ("<end_of_turn>", "<eos>", "<pad>", "<start_of_turn>"):
            out = out.split(stop)[0]
        return out.strip()
    return gen


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-check", action="store_true",
                    help="score the gold as the prediction (must be perfect); offline sanity check")
    ap.add_argument("--pred", type=Path, help="JSONL of model predictions keyed by eval id")
    ap.add_argument("--model", help="run this MLX model as the decomposer over the eval notes")
    ap.add_argument("--adapter", default=None, help="optional LoRA adapter path for --model")
    args = ap.parse_args()

    records = load_eval()
    if args.self_check:
        predictions = {r["id"]: r["gold"] for r in records}
    elif args.model:
        dictionary = json.loads((HERE.parent / "warm" / "dictionary.json").read_text())
        predictions = predict_with_model(records, _mlx_gen(args.model, args.adapter), dictionary)
    elif args.pred:
        predictions = {p["id"]: p for p in
                       (json.loads(line) for line in args.pred.read_text().splitlines() if line.strip())}
    else:
        print("eval_decompose: pass --self-check, --model <path>, or --pred <file>", file=sys.stderr)
        return 2

    rows, agg = score_predictions(records, predictions)
    for r in rows:
        print(f"  {r['id']:26s} {r['shape']:11s} f1={r['fact_f1']:.2f} "
              f"span={r['span_faithfulness']:.2f} disc_r={r['discard_recall']:.2f} "
              f"near_miss={r['near_miss_violations']} fp={r['false_positive_facts']}")
    print(f"\naggregate over {agg.get('n', 0)}: fact_f1={agg.get('fact_f1', 0):.3f} "
          f"span_faithfulness={agg.get('span_faithfulness', 0):.3f} "
          f"discard_recall={agg.get('discard_recall', 0):.3f} "
          f"near_miss_violations={agg.get('near_miss_violations', 0)} "
          f"false_positive_facts={agg.get('false_positive_facts', 0)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
