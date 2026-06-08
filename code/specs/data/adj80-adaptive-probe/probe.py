#!/usr/bin/env python3
"""ADJ80 — adaptive capability probe.

Instead of a monolithic protocol, the framework runs a few cheap CALIBRATION
probes, classifies the model into a capability tier, and SELECTS the decomposition
granularity to match it:

  P1 format-following : can it follow a simple multi-part format instruction?
  P2 structured-output: can it emit exact JSON?
  P3 multi-objective  : can it do a 2-objective-per-item task?

Tier -> protocol (from ADJ76/77/78 evidence):
  fail P1                      -> Tier0 (sub-floor)  -> ATOMIC-NATURAL (single-focus
                                                        steps, lenient parse, framework
                                                        owns all structure)
  pass P1, fail P2 or P3       -> Tier1 (mid)         -> STAGED-NATURAL (few natural steps)
  pass all                     -> Tier2 (capable)     -> MONOLITHIC contract OK

Validation: cross-reference the probe's tier against which protocol actually won
per model in ADJ74 (bare/mono/staged) + ADJ77 (atomic scaffold).
"""
import json
import os
import re
import sys
import urllib.request

GEN = "http://127.0.0.1:11434/api/generate"
MODELS = ["qwen2.5:0.5b", "qwen2.5:1.5b", "qwen2.5:3b", "gemma4:latest", "llama3.1:8b", "qwen2.5:14b"]


def gen(model, prompt, npred=80, timeout=120):
    body = json.dumps({"model": model, "prompt": prompt, "stream": False,
                       "options": {"temperature": 0, "seed": 0, "num_predict": npred}}).encode()
    req = urllib.request.Request(GEN, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())["response"]


def p1_format(model):
    out = gen(model, "Reply with exactly three lines and nothing else.\nLine 1: APPLE\nLine 2: 7\nLine 3: DONE")
    ls = [l.strip() for l in out.strip().splitlines() if l.strip()]
    return len(ls) >= 3 and ls[0].upper().endswith("APPLE") and "7" in ls[1] and ls[2].upper().endswith("DONE")


def p2_json(model):
    out = gen(model, 'Output exactly this JSON and nothing else: {"fruit": "apple", "count": 3}')
    m = re.search(r"\{.*\}", out, re.S)
    if not m:
        return False
    try:
        d = json.loads(m.group())
        return d.get("fruit") == "apple" and d.get("count") == 3
    except Exception:
        return False


def p3_multiobj(model):
    out = gen(model, ("For each item, write the item, then ' = ', then FRUIT or VEG. One per line.\n"
                      "Items: apple, carrot, banana"))
    o = out.lower()
    return (("apple" in o and "fruit" in o.split("carrot")[0] if "carrot" in o else False)
            and re.search(r"carrot\s*=\s*veg", o) is not None
            and re.search(r"banana\s*=\s*fruit", o) is not None)


def tier_and_protocol(p1, p2, p3):
    if not p1:
        return 0, "ATOMIC-NATURAL (single-focus steps; framework owns all structure)"
    if not (p2 and p3):
        return 1, "STAGED-NATURAL (few natural steps; lenient parse)"
    return 2, "MONOLITHIC contract OK (light staging optional)"


# what actually won per model in prior experiments (ADJ74 PS bare/mono/staged, ADJ77 scaffold)
PRIOR = {
    "qwen2.5:0.5b": "monolithic 0.00; atomic/staged ~0.5-0.58 -> needs decomposition",
    "qwen2.5:1.5b": "monolithic 0.42, bare 0.50; free-staging HURT (0.17); v2 scaffold 0.67",
    "qwen2.5:3b": "bare 0.83, monolithic 0.67; v2 scaffold 1.00",
    "gemma4:latest": "bare ~1.00 (handles contract directly)",
    "llama3.1:8b": "bare 0.83 (handles contract directly)",
    "qwen2.5:14b": "bare ~1.00 (handles contract directly)",
}


def main():
    models = sys.argv[1].split(",") if len(sys.argv) > 1 else MODELS
    rows = []
    print(f"{'model':16} {'P1':>4} {'P2':>4} {'P3':>4}  tier  protocol")
    print("-" * 90)
    for m in models:
        try:
            p1, p2, p3 = p1_format(m), p2_json(m), p3_multiobj(m)
        except Exception as e:  # noqa
            print(f"{m:16}  ERROR {e}")
            continue
        tier, proto = tier_and_protocol(p1, p2, p3)
        rows.append({"model": m, "P1": p1, "P2": p2, "P3": p3, "tier": tier, "protocol": proto,
                     "prior_evidence": PRIOR.get(m, "")})
        mark = lambda b: " ok " if b else "FAIL"
        print(f"{m:16} {mark(p1)} {mark(p2)} {mark(p3)}   T{tier}   {proto}")
    print("\nCross-reference vs what actually won per model (ADJ74/ADJ77):")
    for r in rows:
        print(f"  T{r['tier']} {r['model']:16} -> {r['prior_evidence']}")
    json.dump(rows, open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "probe_results.json"), "w"), indent=2)


if __name__ == "__main__":
    main()
