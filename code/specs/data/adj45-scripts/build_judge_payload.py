"""
Pair Style 1/2/3 per benchmark, randomize A/B/C labels per question,
and build the blind-judge payload (one JSON per benchmark).

Outputs:
  /tmp/arms/judge_payload_<benchmark>.json    — what the judge will see (anonymized)
  /tmp/arms/judge_keymap_<benchmark>.json     — the de-anonymization mapping (NOT given to judge)
  /tmp/arms/canonical_<benchmark>.json        — gold references per question
"""
import json, random, hashlib, csv, io, urllib.request
from pathlib import Path

random.seed(42)  # reproducible label randomization

ARMS = Path("/tmp/arms")

def load(name):
    return json.loads((ARMS / name).read_text())

# --------------- MedQA ---------------
def build_medqa():
    s1 = load("medqa_style1.json")
    s2 = load("medqa_style2.json")
    s3 = load("medqa_style3.json")

    # Index by q_idx
    by_idx = {s["q_idx"]: s for s in s1}
    s2_by  = {s["q_idx"]: s for s in s2}
    s3_by  = {s["q_idx"]: s for s in s3}

    # Load canonical from HuggingFace
    url = "https://huggingface.co/datasets/GBaker/MedQA-USMLE-4-options/resolve/main/phrases_no_exclude_test.jsonl"
    raw = urllib.request.urlopen(url).read().decode()
    canonical = []
    for i, line in enumerate(raw.splitlines()):
        if i >= 100:
            break
        canonical.append(json.loads(line))

    payload = []
    keymap = []
    for i in range(100):
        if i not in by_idx or i not in s2_by or i not in s3_by:
            continue
        q = canonical[i]
        candidates_raw = [
            ("style1", by_idx[i].get("choice", ""), by_idx[i].get("rationale", "")),
            ("style2", s2_by[i].get("choice", ""), s2_by[i].get("rationale", "")),
            ("style3", s3_by[i].get("choice", ""), s3_by[i].get("rationale", "")),
        ]
        # Randomize label assignment per question
        labels = ["A", "B", "C"]
        random.shuffle(candidates_raw)
        labeled = list(zip(labels, candidates_raw))
        # Build judge view (no style name)
        candidates_view = [
            {"label": lbl, "choice": choice, "rationale": rat}
            for (lbl, (_style, choice, rat)) in labeled
        ]
        payload.append({
            "q_idx": i,
            "question": q["question"],
            "options": q["options"],
            "candidates": candidates_view,
        })
        keymap.append({
            "q_idx": i,
            "canonical_letter": q["answer_idx"],
            "label_to_style": {lbl: style for (lbl, (style, _, _)) in labeled},
        })

    (ARMS / "judge_payload_medqa.json").write_text(json.dumps(payload, indent=2))
    (ARMS / "judge_keymap_medqa.json").write_text(json.dumps(keymap, indent=2))
    print(f"MedQA: {len(payload)} questions paired and anonymized")

# --------------- SimpleQA ---------------
def build_simpleqa():
    s1 = load("simpleqa_style1.json")
    s2 = load("simpleqa_style2.json")
    s3 = load("simpleqa_style3.json")
    s1_by = {s["q_idx"]: s for s in s1}
    s2_by = {s["q_idx"]: s for s in s2}
    s3_by = {s["q_idx"]: s for s in s3}

    # Load canonical CSV
    url = "https://openaipublic.blob.core.windows.net/simple-evals/simple_qa_test_set.csv"
    raw = urllib.request.urlopen(url).read().decode()
    rdr = csv.DictReader(io.StringIO(raw))
    rows = list(rdr)  # 4326 rows

    payload = []
    keymap = []
    for q_idx in sorted(set(s1_by) & set(s2_by) & set(s3_by)):
        row = rows[q_idx]
        question = row.get("problem") or row.get("question") or ""
        gold = row.get("answer") or ""
        candidates_raw = [
            ("style1", s1_by[q_idx].get("answer", "")),
            ("style2", s2_by[q_idx].get("answer", "")),
            ("style3", s3_by[q_idx].get("answer", "")),
        ]
        labels = ["A", "B", "C"]
        random.shuffle(candidates_raw)
        labeled = list(zip(labels, candidates_raw))
        candidates_view = [
            {"label": lbl, "answer": ans}
            for (lbl, (_style, ans)) in labeled
        ]
        payload.append({
            "q_idx": q_idx,
            "question": question,
            "candidates": candidates_view,
        })
        keymap.append({
            "q_idx": q_idx,
            "canonical_answer": gold,
            "label_to_style": {lbl: style for (lbl, (style, _)) in labeled},
        })
    (ARMS / "judge_payload_simpleqa.json").write_text(json.dumps(payload, indent=2))
    (ARMS / "judge_keymap_simpleqa.json").write_text(json.dumps(keymap, indent=2))
    print(f"SimpleQA: {len(payload)} questions paired and anonymized")

# --------------- TruthfulQA ---------------
def build_truthfulqa():
    """Match by question text (q_idx unreliable across loaders)."""
    s1 = load("truthfulqa_style1.json")
    s2 = load("truthfulqa_style2.json")
    s3 = load("truthfulqa_style3.json")

    def normq(q):
        # Lowercase, collapse whitespace, strip trivial punctuation drift
        import re
        q = q.lower().replace("’", "'").replace("“", '"').replace("”", '"')
        q = re.sub(r"\s+", " ", q).strip()
        q = q.rstrip(".?!")
        return q

    s1_by = {normq(s["question"]): s for s in s1}
    s2_by = {normq(s["question"]): s for s in s2}
    s3_by = {normq(s["question"]): s for s in s3}

    common = sorted(set(s1_by) & set(s2_by) & set(s3_by))

    # Load canonical TruthfulQA CSV (sylinrl/TruthfulQA on GitHub)
    url = "https://raw.githubusercontent.com/sylinrl/TruthfulQA/main/TruthfulQA.csv"
    raw = urllib.request.urlopen(url).read().decode()
    rdr = csv.DictReader(io.StringIO(raw))
    canonical_by = {}
    for row in rdr:
        key = normq(row["Question"])
        canonical_by[key] = {
            "best_answer": row.get("Best Answer", ""),
            "correct_answers": row.get("Correct Answers", ""),
            "incorrect_answers": row.get("Incorrect Answers", ""),
            "source": row.get("Source", ""),
            "category": row.get("Category", ""),
        }

    payload = []
    keymap = []
    matched_canonical = 0
    for k in common:
        canonical = canonical_by.get(k)
        candidates_raw = [
            ("style1", s1_by[k].get("answer", "")),
            ("style2", s2_by[k].get("answer", "")),
            ("style3", s3_by[k].get("answer", "")),
        ]
        labels = ["A", "B", "C"]
        random.shuffle(candidates_raw)
        labeled = list(zip(labels, candidates_raw))
        candidates_view = [
            {"label": lbl, "answer": ans}
            for (lbl, (_style, ans)) in labeled
        ]
        # Use Style 1's original q_idx if available, just for tracking
        q_idx = s1_by[k].get("q_idx")
        payload.append({
            "q_idx": q_idx,
            "question": s1_by[k]["question"],
            "candidates": candidates_view,
        })
        keymap.append({
            "q_idx": q_idx,
            "canonical": canonical if canonical else None,
            "label_to_style": {lbl: style for (lbl, (style, _)) in labeled},
        })
        if canonical:
            matched_canonical += 1

    (ARMS / "judge_payload_truthfulqa.json").write_text(json.dumps(payload, indent=2))
    (ARMS / "judge_keymap_truthfulqa.json").write_text(json.dumps(keymap, indent=2))
    print(f"TruthfulQA: {len(payload)} questions paired (text-matched); canonical found for {matched_canonical}")

build_medqa()
build_simpleqa()
build_truthfulqa()
