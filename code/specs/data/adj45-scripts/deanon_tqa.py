"""De-anonymize TruthfulQA judge results."""
import json, re
from pathlib import Path
from collections import Counter, defaultdict

ARMS = Path("/tmp/arms")
keymap = json.loads((ARMS / "judge_keymap_truthfulqa.json").read_text())
keymap_by_idx = {k["q_idx"]: k for k in keymap}

# Read judge result from transcript
import re as _re
TRANS = Path("/Users/adhithya/.claude/projects/-Users-adhithya-Documents-coding-adventures--claude-worktrees-admiring-goldberg-a988f4/3232e184-a5fb-4f51-8785-71451cf384d3/subagents/agent-ac4d2db4887c6e19b.jsonl")

last_text = None
for line in TRANS.read_text().splitlines():
    if not line.strip(): continue
    try: evt = json.loads(line)
    except: continue
    if evt.get("type") == "assistant":
        for b in evt.get("message",{}).get("content",[]):
            if isinstance(b,dict) and b.get("type")=="text":
                last_text = b.get("text","")

# Extract JSON array
s = last_text.find('[')
for e in range(len(last_text), s, -1):
    chunk = last_text[s:e]
    if not chunk.endswith(']'): continue
    try:
        judge = json.loads(chunk)
        break
    except: continue

print(f"Judge entries: {len(judge)}")

# De-anonymize
per_style = {"style1": {"truthful":0,"informative":0,"best_picks":0,"total":0},
             "style2": {"truthful":0,"informative":0,"best_picks":0,"total":0},
             "style3": {"truthful":0,"informative":0,"best_picks":0,"total":0}}

interesting_qs = []
for j in judge:
    k = keymap_by_idx.get(j["q_idx"])
    if k is None: continue
    label_to_style = k["label_to_style"]
    # tally
    for s in j["scores"]:
        style = label_to_style[s["label"]]
        per_style[style]["total"] += 1
        if s.get("truthful"): per_style[style]["truthful"] += 1
        if s.get("informative"): per_style[style]["informative"] += 1
    best_label = j.get("best_candidate","")
    if best_label.startswith("tie:"):
        winners = best_label[4:].split(",")
        for w in winners:
            w = w.strip()
            if w in label_to_style:
                per_style[label_to_style[w]]["best_picks"] += 1/len(winners)
    else:
        if best_label in label_to_style:
            per_style[label_to_style[best_label]]["best_picks"] += 1
    # Find divergent cases
    distinct = len(set((s.get("truthful"),s.get("informative")) for s in j["scores"]))
    if distinct > 1:
        # Map each label to its style
        details = []
        for s in j["scores"]:
            details.append({"style": label_to_style[s["label"]],
                            "truthful": s.get("truthful"),
                            "informative": s.get("informative")})
        interesting_qs.append({"q_idx": j["q_idx"], "best": label_to_style.get(best_label, best_label),
                              "notes": j.get("notes",""), "details": details})

print("\n=== Per-style aggregate (TruthfulQA, n=29) ===")
print(f"{'Style':<10}{'Truthful':<12}{'Informative':<14}{'Best picks':<12}")
for style in ["style1","style2","style3"]:
    d = per_style[style]
    t,i,b,tot = d['truthful'],d['informative'],d['best_picks'],d['total']
    print(f"{style:<10}{t}/{tot} ({100*t/tot:.0f}%)  {i}/{tot} ({100*i/tot:.0f}%)  {b:.1f}/{len(judge)}")

print("\n=== Divergent cases (where candidates disagreed) ===")
for q in interesting_qs:
    print(f"\nq_idx={q['q_idx']}  best={q['best']}  notes={q['notes']}")
    for d in q['details']:
        print(f"  {d['style']:<10} truthful={d['truthful']} informative={d['informative']}")
