#!/usr/bin/env python3
"""ADJ101 — exercise the CAS: write a verified rulebook -> compile to a program library -> link it
into a held-out input (itself translated to a program) -> execute on CPU with ZERO model calls.

This is the derive-once / reuse-indefinitely loop (paper-2 MYCIN), made concrete:
  1. CAS WRITE  : a byte-accounting-clean, entailment-verified rulebook is canonicalized + content-
                  addressed (sha256) + stored. (Gate, simplified here to the invariant the 100-run
                  already established: every rule source_span is verbatim in the policy.)
  2. COMPILE    : the cached rulebook is compiled into a SELF-CONTAINED python library `rulelib_<hash>.py`
                  (rules baked in + a vendored deterministic evaluator + `decide(facts)`). No repo/model imports.
  3. LINK + RUN : a held-out input is decomposed into typed-IR facts; that IR is emitted AS a small
                  program that `import`s the compiled library and calls `decide(facts)`. Running it
                  produces the verdict + a proof (which rule fired on which fact) — no answer-time model call.

Run: python3 cas_exercise.py
"""
import hashlib
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CAS = os.path.join(HERE, "cas")
for d in ("objects", "lib", "programs"):
    os.makedirs(os.path.join(CAS, d), exist_ok=True)


def norm(s):
    import re
    return re.sub(r"\s+", " ", (s or "")).strip().lower()


# --- 1. CAS WRITE -----------------------------------------------------------------------------------
items = json.load(open(os.path.join(HERE, "items_100.json")))["items"]
ex = {e["idx"]: e for e in json.load(open(os.path.join(HERE, "extractions100.json")))}
TAX1 = next(k for k, it in enumerate(items) if it["id"] == "TAX-1")
policy = items[TAX1]["policy"]
rulebook = ex[TAX1]["rulebook_ir"]

# gate: every rule's source_span must be verbatim in the policy (the byte-accounting the 100-run verified)
pol = norm(policy)
for r in rulebook["rules"]:
    assert norm(r["source_span"]) in pol, f"CAS-write blocked: unverifiable rule {r['id']}"

canon = json.dumps({"rules": rulebook["rules"], "policy": policy}, sort_keys=True, ensure_ascii=False)
digest = hashlib.sha256(canon.encode()).hexdigest()[:16]
obj = {"hash": digest, "domain": items[TAX1]["domain"], "policy": policy, "rules": rulebook["rules"],
       "provenance": "every rule source_span verified verbatim in policy (byte-accounting clean)"}
json.dump(obj, open(os.path.join(CAS, "objects", f"{digest}.json"), "w"), ensure_ascii=False, indent=1)
print(f"[1] CAS WRITE  : rulebook '{items[TAX1]['id']}' -> object {digest} ({len(rulebook['rules'])} rules, byte-verified)")

# --- 2. COMPILE rulebook -> self-contained program library ------------------------------------------
LIB_TEMPLATE = '''# AUTO-GENERATED program library compiled from CAS object {hash}.
# Self-contained: no model, no repo imports. Pure CPU. decide(facts) -> {{verdict, answer, proof}}.
RULES = {rules}
def _eval(v, pred):
    if v is None: return None
    pred = str(pred).strip()
    for op in (">=", "<=", "==", ">", "<"):
        if pred.startswith(op):
            try: x = float(v); n = float(pred[len(op):].strip())
            except (TypeError, ValueError): return False
            return {{">": x>n, "<": x<n, ">=": x>=n, "<=": x<=n, "==": x==n}}[op]
    if pred in ("true","false"): return str(v).strip().lower()==pred
    if pred == "*": return True
    return str(v).strip().lower()==pred.lower()
def decide(facts):
    fired, missing = [], set()
    for r in RULES:
        ok = True
        for slot, pred in (r.get("when") or {{}}).items():
            res = _eval(facts.get(slot), pred)
            if res is None: ok=False; missing.add(slot)
            elif res is False: ok=False
        if ok: fired.append(r)
    thens = {{r["then"] for r in fired}}
    if len(thens)==1:
        r = fired[0]
        return {{"verdict":"DETERMINATE","answer":r["then"],
                 "proof":[{{"rule":r["id"],"fired_on":{{s:facts.get(s) for s in (r.get("when") or {{}})}},"cites":r["source_span"]}}]}}
    if len(thens)>1: return {{"verdict":"CONFLICT","answer":None,"proof":sorted(thens)}}
    return {{"verdict":"INDETERMINATE","answer":None,"missing":sorted(missing)}}
'''
lib_path = os.path.join(CAS, "lib", f"rulelib_{digest}.py")
open(lib_path, "w").write(LIB_TEMPLATE.format(hash=digest, rules=json.dumps(rulebook["rules"], indent=2)))
print(f"[2] COMPILE     : object {digest} -> program library cas/lib/rulelib_{digest}.py")

# --- 3. LINK a held-out input (typed-IR -> program) into the library + run --------------------------
# three held-out cases: the original TAX-1 facts + two NEW ones, decided by the cached library.
held_out = [
    {"id": "TAX-1 (original)", "facts": {"gross_income": 18000}},   # >= 14600 -> required
    {"id": "held-out A",       "facts": {"gross_income": 9500}},    # < 14600  -> not required
    {"id": "held-out B",       "facts": {"gross_income": 14600}},   # == threshold -> required
]
PROG = '''# AUTO-GENERATED input program: the decomposed typed-IR facts, LINKED to the compiled rule library.
import sys, json
sys.path.insert(0, {libdir!r})
from rulelib_{hash} import decide          # <-- link the CAS-compiled rule library
FACTS = {facts}                            # <-- the typed-IR input, translated to data
print(json.dumps(decide(FACTS)))           # CPU only; ZERO answer-time model calls
'''
print(f"[3] LINK + RUN  : held-out inputs (IR->program) import rulelib_{digest} and execute:\\n")
results = []
for case in held_out:
    p = os.path.join(CAS, "programs", "input_%s.py" % case["id"].split()[0].replace("(", ""))
    open(p, "w").write(PROG.format(libdir=os.path.join(CAS, "lib"), hash=digest, facts=json.dumps(case["facts"])))
    out = subprocess.run([sys.executable, p], capture_output=True, text=True, env={"PATH": os.environ.get("PATH", "")})
    r = json.loads(out.stdout)
    results.append({"case": case["id"], "facts": case["facts"], **r})
    proof = r.get("proof")
    print(f"    {case['id']:18} {case['facts']}  ->  {r['verdict']:13} {r.get('answer') or ''}")
    if proof and isinstance(proof, list) and proof and isinstance(proof[0], dict):
        print(f"        proof: rule '{proof[0]['rule']}' fired on {proof[0]['fired_on']} | cites: \"{proof[0]['cites'][:60]}...\"")

json.dump({"cas_object": digest, "library": f"rulelib_{digest}.py", "answer_time_model_calls": 0, "cases": results},
          open(os.path.join(HERE, "cas_exercise_results.json"), "w"), ensure_ascii=False, indent=1)
print(f"\\n[=] derive-once: rulebook compiled ONCE -> {len(held_out)} held-out cases decided on CPU, "
      f"answer-time model calls = 0. Every verdict carries a proof tracing rule -> fact -> policy bytes.")
