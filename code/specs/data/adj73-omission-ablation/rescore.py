#!/usr/bin/env python3
"""Re-score results_raw.json with a STYLE-ROBUST matcher.

The v1 matcher penalized the justified condition because it produces more
natural-language final answers ("No sales tax applies", "$0.00", "0 days",
"cannot be returned at all") rather than bare tokens ("0%", "8%"). This
re-scorer normalizes numbers ($/%/commas, 0.00->0) and adds PER-ITEM
zero/negation handling: a negation cue ("no <noun>", none, free, exempt,
waived, cannot, not eligible, $0, zero) is assigned to accept ONLY if that
item's accept list contains a zero-concept token, else to trap if the trap
list does. This respects items where 'free'/'0' is the TRAP (e.g. PS02).
No model re-run; re-scores saved raw outputs (reproducible).
"""

import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ZERO_TOKENS = {"0", "none", "free", "exempt", "waived", "no fine", "no tax",
               "no discount", "no fee", "no charge", "no return", "cannot",
               "not eligible", "no", "zero"}


def is_num(t):
    return bool(re.fullmatch(r"\d+(?:\.\d+)?", t))


def norm_nums(s):
    out = set()
    for m in re.findall(r"\d+(?:\.\d+)?", s):
        f = float(m)
        out.add(m)
        if f == int(f):
            out.add(str(int(f)))  # 0.00 -> 0, 12.0 -> 12
    return out


def has_zero_token(tokens):
    return any(t.lower() in ZERO_TOKENS or t.lower().startswith("no ") for t in tokens)


NEG_CUE = re.compile(
    r"(\bno\b\s+\w+|\bnone\b|\bfree\b|\bexempt\b|\bwaived\b|\bcannot\b|"
    r"\bnot\s+(eligible|able|allowed|permitted|charged|applicable|returnable)|"
    r"\bzero\b|\$?\s*0(?:\.0+)?\b|\bdoes not\b|\bno further\b)",
    re.IGNORECASE,
)


def hit(answer, tokens):
    a = answer.lower()
    anums = norm_nums(a)
    for t in tokens:
        if is_num(t):
            tn = t
            if float(t) == int(float(t)):
                tn = str(int(float(t)))
            if t in anums or tn in anums:
                return True
        elif t.lower() in a:
            return True
    return False


def score(item, answer):
    acc, trap = item.get("accept", []), item.get("trap", [])
    a_hit, t_hit = hit(answer, acc), hit(answer, trap)
    # per-item negation handling
    if not a_hit and not t_hit and NEG_CUE.search(answer):
        if has_zero_token(acc):
            a_hit = True
        elif has_zero_token(trap):
            t_hit = True
    if item["stratum"] == "PS":
        if a_hit and not t_hit:
            return "correct"
        if a_hit and t_hit:
            return "correct"  # accept present in final answer wins
        if t_hit:
            return "skim"
        return "other"
    else:
        if a_hit:
            return "abstain"
        if t_hit:
            return "fabricate"
        return "other"


def main():
    items = {i["id"]: i for i in json.load(open(os.path.join(HERE, "items.json")))["items"]}
    rows = json.load(open(os.path.join(HERE, "results_raw.json")))
    for r in rows:
        r["class"] = score(items[r["id"]], r["final_answer"])
    json.dump(rows, open(os.path.join(HERE, "results_rescored.json"), "w"), indent=2)

    models = []
    for r in rows:
        if r["model"] not in models:
            models.append(r["model"])
    conds = ["bare", "coverage", "justified"]

    def rate(model, cond, stratum, cls):
        sub = [r for r in rows if r["model"] == model and r["condition"] == cond and r["stratum"] == stratum]
        return sum(1 for r in sub if r["class"] == cls) / len(sub) if sub else None

    print("=" * 80)
    print("RE-SCORED  PS accuracy (override-correct)  |  AB accuracy (abstained)")
    print("=" * 80)
    print(f"{'model':16} {'PS bare':>8} {'PS cov':>7} {'PS just':>8}   {'AB bare':>8} {'AB cov':>7} {'AB just':>8}")
    for m in models:
        ps = [rate(m, c, "PS", "correct") for c in conds]
        ab = [rate(m, c, "AB", "abstain") for c in conds]
        f = lambda x: f"{x:.2f}" if x is not None else "  -"
        print(f"{m:16} {f(ps[0]):>8} {f(ps[1]):>7} {f(ps[2]):>8}   {f(ab[0]):>8} {f(ab[1]):>7} {f(ab[2]):>8}")

    print("\nPS skim-trap rate (lower better):")
    print(f"{'model':16} {'bare':>8} {'coverage':>10} {'justified':>10}")
    for m in models:
        sk = [rate(m, c, "PS", "skim") for c in conds]
        f = lambda x: f"{x:.2f}" if x is not None else "  -"
        print(f"{m:16} {f(sk[0]):>8} {f(sk[1]):>10} {f(sk[2]):>10}")

    print("\nAB fabricate rate (lower better):")
    for m in models:
        fb = [rate(m, c, "AB", "fabricate") for c in conds]
        f = lambda x: f"{x:.2f}" if x is not None else "  -"
        print(f"{m:16} {f(fb[0]):>8} {f(fb[1]):>10} {f(fb[2]):>10}")


if __name__ == "__main__":
    main()
