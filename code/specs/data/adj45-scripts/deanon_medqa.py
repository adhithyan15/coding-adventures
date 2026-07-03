"""De-anonymize MedQA judge results."""
import json
from pathlib import Path

ARMS = Path("/tmp/arms")
keymap = json.loads((ARMS / "judge_keymap_medqa.json").read_text())
keymap_by_idx = {k["q_idx"]: k for k in keymap}

TRANS = Path("/Users/adhithya/.claude/projects/-Users-adhithya-Documents-coding-adventures--claude-worktrees-admiring-goldberg-a988f4/3232e184-a5fb-4f51-8785-71451cf384d3/subagents/agent-a4c88b965574cc3b0.jsonl")
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
    try: judge = json.loads(chunk); break
    except: continue

print(f"Judge entries: {len(judge)}")

per_style = {st: {"correct":0,"rq_sum":0,"best_picks":0.0,"total":0} for st in ["style1","style2","style3"]}

for j in judge:
    k = keymap_by_idx.get(j["q_idx"])
    if k is None: continue
    l2s = k["label_to_style"]
    for s in j["scores"]:
        st = l2s[s["label"]]
        per_style[st]["total"] += 1
        if s.get("correct"): per_style[st]["correct"] += 1
        per_style[st]["rq_sum"] += s.get("reasoning_quality", 0)
    best = j.get("best_candidate","")
    if best.startswith("tie:"):
        winners = [w.strip() for w in best[4:].split(",")]
        for w in winners:
            if w in l2s:
                per_style[l2s[w]]["best_picks"] += 1/len(winners)
    elif best in l2s:
        per_style[l2s[best]]["best_picks"] += 1

print("\n=== MedQA (n=100) ===")
print(f"{'Style':<10}{'Accuracy':<15}{'AvgRQ':<10}{'BestPicks':<12}")
for st in ["style1","style2","style3"]:
    d = per_style[st]
    acc = 100*d['correct']/d['total']
    rq = d['rq_sum']/d['total']
    print(f"{st:<10}{d['correct']}/{d['total']} ({acc:.0f}%)  {rq:.2f}     {d['best_picks']:.1f}")

# Inter-arm agreement on choice
print("\n=== Letter-choice agreement ===")
s1 = json.loads(Path("/tmp/arms/medqa_style1.json").read_text())
s2 = json.loads(Path("/tmp/arms/medqa_style2.json").read_text())
s3 = json.loads(Path("/tmp/arms/medqa_style3.json").read_text())
agree = {"all3":0, "s1_s2":0, "s1_s3":0, "s2_s3":0, "all_diff":0}
by_idx = {x["q_idx"]: x for x in s1}
s2_by = {x["q_idx"]: x for x in s2}
s3_by = {x["q_idx"]: x for x in s3}
for i in range(100):
    c1, c2, c3 = by_idx[i].get("choice"), s2_by[i].get("choice"), s3_by[i].get("choice")
    if c1==c2==c3: agree["all3"] += 1
    elif c1==c2: agree["s1_s2"] += 1
    elif c1==c3: agree["s1_s3"] += 1
    elif c2==c3: agree["s2_s3"] += 1
    else: agree["all_diff"] += 1
for k,v in agree.items(): print(f"  {k}: {v}")
