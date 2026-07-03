"""Build judge-view files containing canonical references (but NOT label→style mapping)."""
import json
from pathlib import Path

ARMS = Path("/tmp/arms")

for bm in ["medqa", "simpleqa", "truthfulqa"]:
    payload = json.loads((ARMS / f"judge_payload_{bm}.json").read_text())
    keymap = json.loads((ARMS / f"judge_keymap_{bm}.json").read_text())

    # Merge canonical reference into payload but strip label_to_style
    judge_view = []
    for p, k in zip(payload, keymap):
        item = dict(p)
        if bm == "medqa":
            item["canonical_letter"] = k["canonical_letter"]
        elif bm == "simpleqa":
            item["canonical_answer"] = k["canonical_answer"]
        elif bm == "truthfulqa":
            item["canonical"] = k["canonical"]
        judge_view.append(item)
    out = ARMS / f"judge_view_{bm}.json"
    out.write_text(json.dumps(judge_view, indent=2))
    print(f"{bm}: {out} ({out.stat().st_size} bytes)")
