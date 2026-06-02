"""Extract final JSON-array result from each agent's transcript jsonl."""
import json, re, sys
from pathlib import Path

AGENTS = {
    "truthfulqa_style1": "ab078b92d3a6920c6",
    "truthfulqa_style2": "a837096245762c272",
    "truthfulqa_style3": "a6ed9211038fe44a1",
    "simpleqa_style1":   "a6c81fdd5b2747eba",
    "simpleqa_style2":   "a148b3576049be2cc",
    "simpleqa_style3":   "a6cfb26c949496ba4",
    "medqa_style1":      "afe65e4f54956d390",
    "medqa_style2":      "ab839dc09e992a378",
    "medqa_style3":      "a543570db6517802d",
}

SUBAGENT_DIR = Path("/Users/adhithya/.claude/projects/-Users-adhithya-Documents-coding-adventures--claude-worktrees-admiring-goldberg-a988f4/3232e184-a5fb-4f51-8785-71451cf384d3/subagents")

def get_final_assistant_text(transcript_path):
    """Return the last assistant message's text content from a jsonl transcript."""
    last_text = None
    with open(transcript_path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                evt = json.loads(line)
            except Exception:
                continue
            if evt.get("type") != "assistant":
                continue
            msg = evt.get("message", {})
            content = msg.get("content", [])
            for block in content:
                if isinstance(block, dict) and block.get("type") == "text":
                    last_text = block.get("text", "")
    return last_text

def extract_json_array(text):
    """Find the largest JSON array in the text."""
    if text is None:
        return None
    # Find first '[' and try to parse from there, looking for largest valid array.
    start = text.find('[')
    if start < 0:
        return None
    # Try parsing from each '[' until success with the longest result
    # Simpler: find the LAST '[' and the LAST ']' and try
    # Even simpler: try the first '[' first.
    # Use json.loads with progressive end positions
    for end in range(len(text), start, -1):
        chunk = text[start:end]
        if not chunk.endswith(']'):
            continue
        try:
            arr = json.loads(chunk)
            if isinstance(arr, list) and len(arr) > 0:
                return arr
        except Exception:
            continue
    return None

for name, aid in AGENTS.items():
    path = SUBAGENT_DIR / f"agent-{aid}.jsonl"
    if not path.exists():
        print(f"MISSING: {name} ({path})", file=sys.stderr)
        continue
    text = get_final_assistant_text(path)
    arr = extract_json_array(text or "")
    if arr is None:
        print(f"PARSE FAIL: {name}", file=sys.stderr)
        continue
    out = Path(f"/tmp/arms/{name}.json")
    out.write_text(json.dumps(arr, indent=2))
    print(f"{name}: {len(arr)} entries -> {out}")
