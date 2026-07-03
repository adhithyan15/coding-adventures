"""De-anonymize SimpleQA judge results."""
import json
from pathlib import Path
from collections import defaultdict

ARMS = Path("/tmp/arms")
keymap = json.loads((ARMS / "judge_keymap_simpleqa.json").read_text())
keymap_by_idx = {k["q_idx"]: k for k in keymap}

# Read judge result
TRANS = Path("/Users/adhithya/.claude/projects/-Users-adhithya-Documents-coding-adventures--claude-worktrees-admiring-goldberg-a988f4/3232e184-a5fb-4f51-8785-71451cf384d3/subagents/agent-a340b5ff85110067c.jsonl")
last_text = None
for line in TRANS.read_text().splitlines():
    if not line.strip(): continue
    try: evt = json.loads(line)
    except: continue
    if evt.get("type") == "assistant":
        for b in evt.get("message",{}).get("content",[]):
            if isinstance(b,dict) and b.get("type")=="text":
                last_text = b.get("text","")
s = last_text.find('[')
for e in range(len(last_text), s, -1):
    chunk = last_text[s:e]
    if not chunk.endswith(']'): continue
    try:
        judge = json.loads(chunk)
        break
    except: continue

print(f"Judge entries: {len(judge)}")

per_style = {st: {"correct":0,"incorrect":0,"refused":0,"partial":0,
                  "cited":0,"best_picks":0,"total":0}
             for st in ["style1","style2","style3"]}

for j in judge:
    k = keymap_by_idx.get(j["q_idx"])
    if k is None: continue
    l2s = k["label_to_style"]
    for s in j["scores"]:
        st = l2s[s["label"]]
        per_style[st]["total"] += 1
        v = s.get("verdict","")
        if v == "correct": per_style[st]["correct"] += 1
        elif v == "incorrect": per_style[st]["incorrect"] += 1
        elif v == "refused": per_style[st]["refused"] += 1
        elif v == "partial": per_style[st]["partial"] += 1
        if s.get("cited"): per_style[st]["cited"] += 1
    best = j.get("best_candidate","")
    if best.startswith("tie:"):
        winners = best[4:].split(",")
        for w in winners:
            w = w.strip()
            if w in l2s:
                per_style[l2s[w]]["best_picks"] += 1/len(winners)
    elif best in l2s:
        per_style[l2s[best]]["best_picks"] += 1

print("\n=== Per-style aggregate (SimpleQA, n=100) ===")
print(f"{'Style':<10}{'Correct':<12}{'Wrong':<10}{'Refused':<10}{'Partial':<10}{'Cited':<10}{'Best':<10}")
for st in ["style1","style2","style3"]:
    d = per_style[st]
    print(f"{st:<10}{d['correct']:<12}{d['incorrect']:<10}{d['refused']:<10}{d['partial']:<10}{d['cited']:<10}{d['best_picks']:<10.1f}")

# Calibration: hallucination rate on attempted = wrong / (correct + wrong + partial)
print("\n=== Calibration: hallucination rate on attempted ===")
for st in ["style1","style2","style3"]:
    d = per_style[st]
    attempted = d['correct'] + d['incorrect'] + d['partial']
    hall_rate = d['incorrect'] / attempted if attempted else 0
    print(f"{st}: {d['incorrect']}/{attempted} = {hall_rate*100:.1f}% hallucination on attempted")
