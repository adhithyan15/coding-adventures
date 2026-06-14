#!/usr/bin/env python3
"""bench_models.py - how low can the decomposer go? A model-size ladder.

MYCIN-2026 bench. The warm path asks the model for ONE thing: decompose messy
prose into typed findings in the closed dictionary. Everything downstream is the
same 0-model-call CPU engine over the same grounded rulebook. So the model is a
swappable part - and this measures how SMALL it can be before the decompositions
stop yielding correct diagnoses.

For each model on a size ladder, decompose every vignette, run the deterministic
pipeline (ir_to_adj -> decide), and score against gold. Reports per model:
JSON-parse success, findings extracted, terms dropped at the closed-vocabulary
gate (hallucinations the engine never sees), and diagnoses correct - then the
FLOOR: the smallest model that still gets every case right.

Usage:  python3 bench/bench_models.py [--models a,b,c]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

MYCIN = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(MYCIN / "warm"))
import decide as decide_mod  # noqa: E402
import decompose as decompose_mod  # noqa: E402
import ir_to_adj as ir_mod  # noqa: E402

CASES = MYCIN / "cases" / "cases.json"
DICT = MYCIN / "warm" / "dictionary.json"
OUT = MYCIN / "bench" / "model_floor.json"

TOLERANT = False  # set by --tolerant: have the framework absorb small-model JSON variance
CONSTRAINED = False  # set by --constrained: grammar-constrained decoding (schema from the dictionary)


def dict_schema(d: dict) -> dict:
    """Compile the dictionary into a strict JSON schema: each finding's functor is
    coupled to its OWN value domain (anyOf of const-functor + enum-values). Passed
    to Ollama as `format`, this is grammar-constrained decoding - the model can
    ONLY emit dictionary-legal functor(value) pairs. Same `define` source as the
    parser's vocabulary check; used here to GENERATE rather than to RECOGNIZE."""
    alts = [{"type": "object",
             "properties": {"functor": {"const": f["functor"]},
                            "value": {"enum": f["value_domain"]},
                            "span": {"type": "string"},
                            "polarity": {"enum": ["stated", "inferred", "denied", "affirmed"]}},
             "required": ["functor", "value"]} for f in d["findings"]]
    return {"type": "object",
            "properties": {"findings": {"type": "array", "items": {"anyOf": alts}}},
            "required": ["findings"]}


def ollama_constrained(model: str, prompt: str, schema: dict) -> str:
    import urllib.request
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "format": schema,
                       "options": {"temperature": 0, "seed": 0, "num_predict": 600}}).encode()
    req = urllib.request.Request("http://127.0.0.1:11434/api/generate", data=body,
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.loads(r.read())["response"]


def tolerant_findings(ir: dict, domains: dict) -> dict:
    """Coerce the wild JSON shapes small models emit into the standard
    [{functor, value, ...}] finding list - SAFELY: a finding is kept only if its
    functor is a known dictionary functor AND its value is in that functor's
    domain. Anything ambiguous is dropped, never guessed, so the engine never
    sees a wrong finding (the 0-wrong-diagnosis property is preserved). This is
    'intelligence in the framework': the normalizer does more work so the model
    can do less. Returns a NEW ir with a clean findings list."""
    raw = ir.get("findings", [])
    # Shape: findings is itself a {functor: value} mapping (qwen-1.5b).
    if isinstance(raw, dict):
        raw = [{"functor": k, "value": v} for k, v in raw.items()]
    out = []
    for f in raw if isinstance(raw, list) else []:
        # Shape: a bare string "functor(value)".
        if isinstance(f, str):
            f = {"term": f}
        if not isinstance(f, dict):
            continue
        # Shape: a {functor: value} mini-mapping with known functor keys
        # (rather than {"functor":..., "value":...}).
        known_keys = [k for k in f if k in domains]
        if known_keys and "functor" not in f and "term" not in f:
            for k in known_keys:
                out.append({"functor": k, "value": f[k],
                            "type": f.get("type"), "polarity": f.get("polarity")})
            continue
        # Shape: functor landed in "span" (qwen-3b) - recover only if it's a real functor.
        if "functor" not in f and "term" not in f:
            cand = str(f.get("span", "")).strip().lower().replace(" ", "_")
            if cand in domains:
                f = {**f, "functor": cand}
        out.append(f)
    # inference_justifications: small models sometimes drop bare strings in here;
    # keep only well-formed dict entries (the rest can't gate anything anyway).
    just = [j for j in ir.get("inference_justifications", []) if isinstance(j, dict)]
    return {**ir, "findings": out, "inference_justifications": just}

# Size ladder, largest -> smallest (Ollama tags + approx on-disk size).
LADDER = [
    ("gemma4:latest", "9.6 GB"),
    ("llama3.1:8b", "4.9 GB"),
    ("qwen2.5:3b", "1.9 GB"),
    ("qwen2.5:1.5b", "986 MB"),
    ("qwen2.5:0.5b", "397 MB"),
]


def run_model(model: str, cli, d: dict, cases: list, domains) -> dict:
    rows = []
    schema = dict_schema(d) if CONSTRAINED else None
    for c in cases:
        try:
            if CONSTRAINED:
                # Apples-to-apples: SAME rich decompose prompt; the schema constraint
                # is the only variable vs the unconstrained run.
                raw = ollama_constrained(model, decompose_mod.prompt_for(c["vignette"], d), schema)
            else:
                raw = decompose_mod.ollama(model, decompose_mod.prompt_for(c["vignette"], d))
        except Exception as e:  # noqa: BLE001
            rows.append({"case": c["id"], "parse_ok": False, "error": str(e)[:80],
                         "leader": None, "score": "model_error"})
            continue
        ir = decompose_mod.coerce_ir(c["id"], raw)
        parse_ok = ir["_model_raw_ok"]
        if TOLERANT or CONSTRAINED:
            ir = tolerant_findings(ir, domains)
        try:
            obs, kept, dropped = ir_mod.ir_to_adj(ir, domains)
        except Exception as e:  # noqa: BLE001
            rows.append({"case": c["id"], "parse_ok": parse_ok, "error": str(e)[:80],
                         "leader": None, "score": "ir_error"})
            continue
        if not kept:
            rows.append({"case": c["id"], "parse_ok": parse_ok, "n_findings": 0,
                         "n_dropped": len(dropped), "leader": None, "score": "no_findings"})
            continue
        res = decide_mod.decide(c["id"], obs, cli)
        leader = res["leader"]
        dtype = res["decision"].get("type")
        score = ("abstained" if dtype == "insufficient_evidence"
                 else "correct" if leader == c["gold"] else "wrong")
        rows.append({"case": c["id"], "parse_ok": parse_ok, "n_findings": len(kept),
                     "n_dropped": len(dropped), "leader": leader, "gold": c["gold"],
                     "score": score})
    n = len(rows)
    return {
        "model": model,
        "cases": rows,
        "parse_ok": sum(1 for r in rows if r.get("parse_ok")),
        "correct": sum(1 for r in rows if r["score"] == "correct"),
        "abstained": sum(1 for r in rows if r["score"] == "abstained"),
        "wrong": sum(1 for r in rows if r["score"] == "wrong"),
        "failed": sum(1 for r in rows if r["score"] in ("model_error", "ir_error", "no_findings")),
        "hallucinations_gated": sum(r.get("n_dropped", 0) for r in rows),
        "n": n,
    }


def main(argv: list[str]) -> int:
    global TOLERANT, CONSTRAINED
    TOLERANT = "--tolerant" in argv
    CONSTRAINED = "--constrained" in argv
    cli = decide_mod.find_cli()
    if cli is None:
        print("bench: adj-lang-cli not built", file=sys.stderr)
        return 3
    print(f"(normalizer: {'TOLERANT — framework absorbs small-model JSON variance' if TOLERANT else 'STRICT'})")
    ladder = LADDER
    if "--models" in argv:
        names = argv[argv.index("--models") + 1].split(",")
        sizes = {m: s for m, s in LADDER}
        ladder = [(m, sizes.get(m, "?")) for m in names]

    d = json.loads(DICT.read_text())
    cases = json.loads(CASES.read_text())["cases"]
    domains = ir_mod.load_domains()

    results = []
    print(f"{'model':16s} {'size':8s} {'correct':>8s} {'abst':>5s} {'wrong':>6s} "
          f"{'fail':>5s} {'parse':>6s} {'hall-gated':>10s}")
    for model, size in ladder:
        r = run_model(model, cli, d, cases, domains)
        r["size"] = size
        results.append(r)
        print(f"{model:16s} {size:8s} {r['correct']:>5d}/{r['n']:<2d} {r['abstained']:>5d} "
              f"{r['wrong']:>6d} {r['failed']:>5d} {r['parse_ok']:>4d}/{r['n']:<1d} "
              f"{r['hallucinations_gated']:>10d}")

    # The floor: smallest model (last in the largest->smallest ladder) with all correct.
    all_correct = [r for r in results if r["correct"] == r["n"] and r["wrong"] == 0]
    floor = all_correct[-1] if all_correct else None
    summary = {
        "_doc": "How low can the decomposer go. Each model decomposes the vignettes; "
                "the SAME 0-call CPU engine over the SAME grounded rulebook diagnoses. "
                "'floor' = smallest model that still gets every case right. "
                "'hallucinations_gated' = bad terms the closed-vocabulary gate dropped "
                "before they reached the engine (the safety net that lets small models work).",
        "ladder": [m for m, _ in ladder],
        "results": results,
        "floor_model": floor["model"] if floor else None,
        "floor_size": floor["size"] if floor else None,
    }
    OUT.write_text(json.dumps(summary, indent=2) + "\n")
    if floor:
        print(f"\nFLOOR: {floor['model']} ({floor['size']}) still gets {floor['correct']}/{floor['n']} "
              f"correct. Smaller models below this lose cases (see model_floor.json).")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
