# AUTO-GENERATED program library compiled from CAS object ff22eace640aba13.
# Self-contained: no model, no repo imports. Pure CPU. decide(facts) -> {verdict, answer, proof}.
RULES = [
  {
    "id": "must-file",
    "when": {
      "gross_income": ">= 14600"
    },
    "then": "REQUIRED_TO_FILE",
    "source_span": "An individual must file a federal income tax return for the year if their gross income equals or exceeds the filing threshold of $14,600."
  },
  {
    "id": "no-return-below-threshold",
    "when": {
      "gross_income": "< 14600"
    },
    "then": "NOT_REQUIRED_TO_FILE",
    "source_span": "If gross income is below the threshold, no return is required."
  }
]
def _eval(v, pred):
    if v is None: return None
    pred = str(pred).strip()
    for op in (">=", "<=", "==", ">", "<"):
        if pred.startswith(op):
            try: x = float(v); n = float(pred[len(op):].strip())
            except (TypeError, ValueError): return False
            return {">": x>n, "<": x<n, ">=": x>=n, "<=": x<=n, "==": x==n}[op]
    if pred in ("true","false"): return str(v).strip().lower()==pred
    if pred == "*": return True
    return str(v).strip().lower()==pred.lower()
def decide(facts):
    fired, missing = [], set()
    for r in RULES:
        ok = True
        for slot, pred in (r.get("when") or {}).items():
            res = _eval(facts.get(slot), pred)
            if res is None: ok=False; missing.add(slot)
            elif res is False: ok=False
        if ok: fired.append(r)
    thens = {r["then"] for r in fired}
    if len(thens)==1:
        r = fired[0]
        return {"verdict":"DETERMINATE","answer":r["then"],
                 "proof":[{"rule":r["id"],"fired_on":{s:facts.get(s) for s in (r.get("when") or {})},"cites":r["source_span"]}]}
    if len(thens)>1: return {"verdict":"CONFLICT","answer":None,"proof":sorted(thens)}
    return {"verdict":"INDETERMINATE","answer":None,"missing":sorted(missing)}
