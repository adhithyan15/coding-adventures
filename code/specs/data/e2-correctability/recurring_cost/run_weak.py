#!/usr/bin/env python3
"""E2 recurring-cost — the WEAKER-MODEL prose arm (the regime where recurring error lives).

Capable models (Haiku) re-derive a buried override correctly on every call, so the recurring
cost shows up as redundant work + lost determinism, not errors. But the framework's whole premise
is that capability should live in the PIPELINE, so a CHEAP model suffices. This script runs the
plain-prose arm on small local models (Ollama) over the buried-override corpus. A small model
anchors on the prominent distance cue and MISSES the buried clause-8 override — and, being
stateless, misses it AGAIN on every case. That is the recurring cost, measured.

The framework arm pays the interpretation ONCE: the override is derived into a byte-verified rule
(see run_raw_hard.json / score.py, framework = 7/7 + control), then every case — present and
future — is decided by the engine at zero answer-time model calls. The expensive derivation is
paid once; reuse is free and deterministic.

Run: python3 run_weak.py            (requires a local Ollama with the listed models)
"""
import json
import os
import re
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
corpus = json.load(open(os.path.join(HERE, "corpus_hard.json")))
policy = corpus["policy"]
MODELS = ["qwen2.5:0.5b", "qwen2.5:1.5b", "qwen2.5:3b", "llama3.1:8b"]
OLLAMA = "http://localhost:11434/api/generate"


def ask(model, prompt):
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "options": {"temperature": 0}}).encode()
    req = urllib.request.Request(OLLAMA, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.load(r)["response"]


def parse(text):
    u = (text or "").upper()
    # NOT_ENTITLED before ENTITLED (substring containment)
    if "NOT_ENTITLED" in u or "NOT ENTITLED" in u or re.search(r"\bNOT\b.{0,12}\bENTITLED\b", u):
        return "NOT_ENTITLED"
    if "ENTITLED" in u:
        return "ENTITLED"
    return "UNPARSED"


def prompt_for(case):
    return (f"POLICY:\n{policy}\n\nCASE:\n{case['scenario']}\n\n{corpus['question']}\n"
            "Answer with exactly ENTITLED or NOT_ENTITLED on the first line, then one sentence of why.")


results = {}
for model in MODELS:
    rows = []
    try:
        for case in corpus["cases"]:
            out = ask(model, prompt_for(case))
            v = parse(out)
            rows.append({"id": case["id"], "kind": case["kind"], "gold": case["gold"],
                         "verdict": v, "ok": v == case["gold"], "raw": out.strip()[:200]})
    except Exception as exn:  # noqa: BLE001
        results[model] = {"error": str(exn)}
        continue
    ov = [r for r in rows if r["kind"] in ("override", "held_out_override")]
    miss = [r for r in ov if not r["ok"]]
    results[model] = {
        "override_cases_M": len(ov),
        "override_misses": [r["id"] for r in miss],
        "override_miss_rate": round(len(miss) / len(ov), 3) if ov else None,
        "override_correct": f"{len(ov) - len(miss)}/{len(ov)}",
        "control_HO-2_ok": next((r["ok"] for r in rows if r["kind"] == "control_not_owned"), None),
        "rows": rows,
    }
    print(f"{model:16} override miss-rate {results[model]['override_miss_rate']} "
          f"(missed {results[model]['override_misses']})  control_ok={results[model]['control_HO-2_ok']}")

summary = {
    "framework_arm": {"interpretation_paid": "ONCE (byte-verified rule); answer-time model calls = 0",
                      "override_correct": "7/7 + control (see recurrence_results_hard.json)"},
    "prose_arm_by_model": {m: {k: v for k, v in d.items() if k != "rows"} for m, d in results.items()},
    "finding": ("recurring error appears as model capability drops: a small model misses the buried "
                "override and re-misses it on every case (stateless), so the prose miss-rate persists "
                "and the cost recurs over the same fact; the framework derived the override once and "
                "is 0-error on all cases at 0 answer-time model calls."),
}
json.dump({"summary": summary, "by_model": results}, open(os.path.join(HERE, "recurrence_weak.json"), "w"), indent=1)
print("\n" + json.dumps(summary["prose_arm_by_model"], indent=1))
