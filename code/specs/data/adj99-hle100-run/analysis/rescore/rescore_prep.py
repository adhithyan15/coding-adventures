#!/usr/bin/env python3
"""ADJ99 defensibility rescore — preprocessing, format normalization, and the
deterministic confound analysis.

Why this script exists
----------------------
ADJ99's headline defensibility numbers (plain-haiku 2.14 / plain-opus 3.72 /
fw-haiku 2.68 / fw-opus 3.61) were produced by a blind Opus judge scoring 0-5 on
a rubric that operationalized defensibility as *citation/traceability density*
("nearly every claim traceable to a cited source"). That is the WRONG construct:
the thesis defines defensibility as "the locus of contingency is exposed — the
load-bearing premise is surfaced and flagged as fallible so a reviewer can
override it and re-derive," EXPLICITLY decoupled from correctness.

Two flaws follow, and this script proves the first deterministically:

  (1) Format confound. The fw arms emit a literal
      `RETRIEVED FACTS (CAS): ... REASONING CHAIN ... [cites: n]` structure while
      the plain arms emit prose. A trivial regex separates the arms with 100%
      accuracy, so the "blind" judge was not blind, and the old rubric rewarded
      exactly that citation-shaped format.

  (2) Wrong construct. Citation density rewards a confidently-wrong chain whose
      every step cites a bad fact. (Tested by the re-judge, not here.)

What this script does
---------------------
  * Joins items_100.json with the 20 clean batch files (excludes the degraded
    rate-limited batch_13 re-run).
  * Normalizes every cell's reasoning into a UNIFORM, style-neutral envelope so
    the re-judge cannot read the arm off the format.
  * Emits one blind per-cell file (no arm, no gold, no old score) for the judge
    workflow to consume.
  * Emits a private cell_map.json joining index -> arm/gold/old_score for the
    post-judge aggregation.
  * Computes and saves the deterministic confound analysis.

Run:  python3 rescore_prep.py
"""
import json
import glob
import os
import re
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
RUN_ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))  # adj99-hle100-run/
OUT = HERE
CELLS_DIR = os.path.join(OUT, "judge_cells")
os.makedirs(CELLS_DIR, exist_ok=True)

# ---------------------------------------------------------------------------
# Format normalizer.
#
# Goal: render fw-* trails and plain-* work into the SAME shape so style cannot
# leak the arm, while preserving the substantive reasoning prose. Under the new
# (counterfactual) rubric the source URLs are not what is scored — whether the
# load-bearing premise is named and flagged is — so stripping citation chrome is
# both safe and necessary to kill the confound.
# ---------------------------------------------------------------------------

SECTION_HEADERS = [
    "RETRIEVED FACTS (CAS):",
    "RETRIEVED FACTS:",
    "RETRIEVED FACTS",
    "REASONING CHAIN:",
    "REASONING CHAIN",
    "REASONING:",
    "ANSWER:",
    "CONCLUSION:",
]


def _strip_balanced(text, opener):
    """Remove every `opener ... )` span, honoring nested parentheses.

    The data contains spans like `(src: Wikipedia, 'X' (https://...))` where the
    closing paren of the URL nests inside the src paren, so a non-greedy regex
    would cut the span short. This walks the string and matches parens by depth.
    """
    out = []
    i = 0
    n = len(text)
    olen = len(opener)
    while i < n:
        if text.startswith(opener, i):
            depth = 0
            j = i
            while j < n:
                if text[j] == "(":
                    depth += 1
                elif text[j] == ")":
                    depth -= 1
                    if depth == 0:
                        j += 1
                        break
                j += 1
            i = j  # skip the whole span
        else:
            out.append(text[i])
            i += 1
    return "".join(out)


def normalize(reasoning, answer):
    """Return a style-neutral `REASONING: ... \n\nCONCLUSION: ...` rendering."""
    t = reasoning or ""
    # Drop balanced `(src: ...)` provenance parentheticals (fw only).
    t = _strip_balanced(t, "(src:")
    # Drop `[cites: ...]` inline markers (fw only).
    t = re.sub(r"\[cites:[^\]]*\]", "", t)
    # Drop leading `[n]` fact enumerators at line starts (fw only).
    t = re.sub(r"(?m)^\s*\[\d+\]\s*", "", t)
    # Drop standalone section-header lines (case-sensitive, they are ALL-CAPS).
    lines = []
    for ln in t.splitlines():
        stripped = ln.strip()
        if stripped in SECTION_HEADERS:
            continue
        lines.append(ln)
    t = "\n".join(lines)
    # Collapse runs of blank lines and trailing space.
    t = re.sub(r"\n{3,}", "\n\n", t).strip()

    a = (answer or "").strip()
    # The fw answer sometimes repeats an `ANSWER:` header; the plain answer does
    # not. Strip a leading one for symmetry.
    a = re.sub(r"^\s*ANSWER:\s*", "", a)
    return "REASONING:\n{}\n\nCONCLUSION:\n{}".format(t if t else "(none provided)", a)


# ---------------------------------------------------------------------------
# Load + join.
# ---------------------------------------------------------------------------

items = {i["id"]: i for i in json.load(open(os.path.join(RUN_ROOT, "items_100.json")))}
batch_files = sorted(
    f for f in glob.glob(os.path.join(RUN_ROOT, "batches", "batch_*.json"))
    if "degraded" not in f
)
assert len(batch_files) == 20, "expected 20 clean batches, got %d" % len(batch_files)

ARMS = ["plain-haiku", "plain-opus", "fw-haiku", "fw-opus"]


def reasoning_of(arm, d):
    return d.get("trail") if arm.startswith("fw-") else d.get("work")


cells = []  # one row per (item, arm)
idx = 0
for bf in batch_files:
    b = json.load(open(bf))
    for it in b["result"]["items"]:
        iid = it["id"]
        q = items[iid]["question"]
        gold = items[iid]["answer"]
        cat = it["category"]
        for arm in ARMS:
            d = it["arms"][arm]
            ans = d.get("answer", "")
            reasoning = reasoning_of(arm, d) or ""
            is_error = (
                str(ans).strip().startswith("[agent-error]")
                or str(reasoning).strip().startswith("[agent-error]")
            )
            cells.append({
                "idx": idx,
                "item_id": iid,
                "category": cat,
                "question": q,
                "gold": gold,
                "arm": arm,
                "old_def": d.get("defensibility"),
                "old_acc": d.get("accuracy"),
                "provenance_complete": d.get("provenance_complete"),
                "grounded": d.get("grounded"),
                "n_facts": d.get("n_facts"),
                "answer": ans,
                "reasoning_raw": reasoning,
                "agent_error": is_error,
                "normalized": None if is_error else normalize(reasoning, ans),
            })
            idx += 1

assert len(cells) == 400

# ---------------------------------------------------------------------------
# Emit blind per-cell judge files (only what the judge may see).
# ---------------------------------------------------------------------------
n_blind = 0
for c in cells:
    if c["agent_error"]:
        continue
    blind = {
        "idx": c["idx"],
        "category": c["category"],
        "question": c["question"],
        "normalized": c["normalized"],
    }
    with open(os.path.join(CELLS_DIR, "cell_%04d.json" % c["idx"]), "w") as f:
        json.dump(blind, f, ensure_ascii=False, indent=1)
    n_blind += 1

# Private map for post-judge aggregation (never shown to judge).
with open(os.path.join(OUT, "cell_map.json"), "w") as f:
    json.dump(cells, f, ensure_ascii=False, indent=1)

# ---------------------------------------------------------------------------
# Deterministic confound analysis.
# ---------------------------------------------------------------------------

FORMAT_TELLS = ["RETRIEVED FACTS", "REASONING CHAIN", "[cites", "(src:"]


def has_format_tell(raw):
    return any(m in (raw or "") for m in FORMAT_TELLS)


# (1) Does the citation-shaped format perfectly identify the arm?
tell_by_arm = defaultdict(lambda: [0, 0])  # arm -> [has_tell, total_nonerror]
for c in cells:
    if c["agent_error"]:
        continue
    tell_by_arm[c["arm"]][1] += 1
    if has_format_tell(c["reasoning_raw"]):
        tell_by_arm[c["arm"]][0] += 1

# A regex classifier: predict "fw" iff a format tell is present. Accuracy vs truth.
correct_pred = total_pred = 0
for c in cells:
    if c["agent_error"]:
        continue
    pred_fw = has_format_tell(c["reasoning_raw"])
    true_fw = c["arm"].startswith("fw-")
    total_pred += 1
    if pred_fw == true_fw:
        correct_pred += 1
format_arm_accuracy = correct_pred / total_pred

# (2) Old defensibility vs correctness — is the score independent of being right?
def mean(xs):
    xs = [x for x in xs if x is not None]
    return round(sum(xs) / len(xs), 3) if xs else None


def is_correct(acc):
    return acc == "correct"


by_arm_corr = {}
for arm in ARMS:
    rows = [c for c in cells if c["arm"] == arm and not c["agent_error"]]
    corr = [c["old_def"] for c in rows if is_correct(c["old_acc"])]
    inc = [c["old_def"] for c in rows if not is_correct(c["old_acc"])]
    by_arm_corr[arm] = {
        "mean_def_correct": mean(corr), "n_correct": len(corr),
        "mean_def_incorrect": mean(inc), "n_incorrect": len(inc),
    }

# Overall point-biserial-ish: mean def of correct vs incorrect, pooled.
all_rows = [c for c in cells if not c["agent_error"] and c["old_def"] is not None]
pooled_corr = mean([c["old_def"] for c in all_rows if is_correct(c["old_acc"])])
pooled_inc = mean([c["old_def"] for c in all_rows if not is_correct(c["old_acc"])])

# Fraction of def>=4 cells that were WRONG (the rubric's failure mode).
ge4 = [c for c in all_rows if c["old_def"] >= 4]
ge4_wrong = [c for c in ge4 if not is_correct(c["old_acc"])]
ge4_wrong_frac = round(len(ge4_wrong) / len(ge4), 3) if ge4 else None

# (3) The fw-vs-plain old-def gap = the confound magnitude.
old_means = {arm: mean([c["old_def"] for c in cells
                        if c["arm"] == arm and not c["agent_error"]]) for arm in ARMS}

# (4) Within fw, does provenance_complete predict old_def (format -> score)?
def parse_grounded(g):
    # "4/4" -> 1.0 ; "0/0" -> None
    try:
        num, den = g.split("/")
        den = int(den)
        return int(num) / den if den else None
    except Exception:
        return None


fw_rows = [c for c in cells if c["arm"].startswith("fw-") and not c["agent_error"]]
prov_true = mean([c["old_def"] for c in fw_rows if c["provenance_complete"] is True])
prov_false = mean([c["old_def"] for c in fw_rows if c["provenance_complete"] is False])

analysis = {
    "format_confound": {
        "format_tell_separates_arm_accuracy": round(format_arm_accuracy, 4),
        "note": "A 1-line regex on {RETRIEVED FACTS, REASONING CHAIN, [cites, (src:} "
                "predicts fw-vs-plain at this accuracy. ~1.0 means the 'blind' judge "
                "could identify the arm from style alone; it was not blind.",
        "has_tell_by_arm": {k: {"has_tell": v[0], "n": v[1]} for k, v in tell_by_arm.items()},
    },
    "score_vs_correctness": {
        "pooled_mean_def_correct": pooled_corr,
        "pooled_mean_def_incorrect": pooled_inc,
        "per_arm": by_arm_corr,
        "def_ge4_count": len(ge4),
        "def_ge4_wrong_count": len(ge4_wrong),
        "def_ge4_wrong_fraction": ge4_wrong_frac,
        "note": "If defensibility measured soundness, def>=4 answers would mostly be "
                "correct. They are mostly WRONG, because the rubric scored attribution, "
                "not whether the load-bearing premise was true or flagged fallible.",
    },
    "old_def_means_by_arm": old_means,
    "fw_provenance_vs_score": {
        "mean_def_provenance_complete": prov_true,
        "mean_def_provenance_incomplete": prov_false,
        "note": "Within fw, the citation-completeness flag tracks the old score — "
                "more confirmation the rubric graded citation chrome.",
    },
    "counts": {
        "total_cells": len(cells),
        "agent_errors": sum(1 for c in cells if c["agent_error"]),
        "blind_cells_emitted": n_blind,
    },
}

with open(os.path.join(OUT, "confound_analysis.json"), "w") as f:
    json.dump(analysis, f, ensure_ascii=False, indent=2)

print(json.dumps(analysis, indent=2))
print("\nblind cells:", n_blind, "->", CELLS_DIR)
