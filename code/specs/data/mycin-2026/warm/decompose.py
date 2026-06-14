#!/usr/bin/env python3
"""decompose.py - the ONE model touchpoint of the warm path. Decompose-only.

MYCIN-2026 M6. A local model (Ollama; default llama3.1:8b) maps a messy clinical
vignette into TYPED FINDINGS drawn from the closed dictionary - and nothing else.
It does NOT diagnose, rank, or weigh evidence; that is the CPU engine's job
(decide.py), at 0 answer-time model calls. The model is constrained to the
dictionary's functors, value domains, and surface forms, so its output IR shares
one vocabulary with the rulebook by construction.

Output per case: ir/<id>.json
  { case_id,
    findings: [{ term: "functor(value)", span: <verbatim phrase>, type: stated|inferred,
                 polarity: affirmed|denied }],
    discard:  [{ span, reason }],            # prose not mapped to any term
    inference_justifications: [{ term, basis_span, verdict: ENTAILED|LEAP }] }

Usage:  python3 decompose.py [--model llama3.1:8b]
"""

from __future__ import annotations

import json
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DICT = ROOT / "warm" / "dictionary.json"
CASES = ROOT / "cases" / "cases.json"
IR_DIR = ROOT / "ir"
OLLAMA = "http://127.0.0.1:11434/api/generate"


def ollama(model: str, prompt: str) -> str:
    body = json.dumps({
        "model": model,
        "prompt": prompt,
        "stream": False,
        "format": "json",
        "options": {"temperature": 0, "seed": 0, "num_predict": 1200},
    }).encode()
    req = urllib.request.Request(OLLAMA, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.loads(r.read())["response"]


def vocab_block(d: dict) -> str:
    lines = ["LEGAL FINDING TERMS (use term(value); value MUST be one of the listed domain):"]
    for f in d["findings"]:
        surfaces = "; ".join(f["surfaces"][:4])
        lines.append(f'  {f["functor"]}(one of {f["value_domain"]}) - prose like: {surfaces}')
    lines.append("Hypotheses (do NOT output these as findings, do NOT diagnose): "
                 + ", ".join(h["name"] for h in d["hypotheses"]))
    return "\n".join(lines)


def prompt_for(vignette: str, d: dict) -> str:
    return f"""You are a clinical DECOMPOSER. Map the vignette below to typed findings from a
CLOSED vocabulary. You are NOT a diagnostician: do not name or rank a diagnosis,
do not weigh evidence. Only extract findings.

{vocab_block(d)}

RULES:
- Output every finding the vignette states, as "functor(value)" using ONLY the
  legal terms and their allowed values. A finding explicitly present -> type
  "stated"; a finding you had to infer -> type "inferred" (and add an entry to
  inference_justifications with verdict ENTAILED if the prose forces it, LEAP if
  it is a guess). A finding explicitly absent/negative -> polarity "denied".
- "span" is the VERBATIM phrase from the vignette that the finding came from.
- Put prose you could not map into "discard" with a short reason.
- Map a normal/negative result to its (normal)/(negative) value, not by omission.

VIGNETTE:
{vignette}

Output ONLY a JSON object with keys: findings, discard, inference_justifications."""


def coerce_ir(case_id: str, raw: str) -> dict:
    try:
        obj = json.loads(raw)
    except json.JSONDecodeError:
        obj = {}
    return {
        "case_id": case_id,
        "findings": obj.get("findings", []) if isinstance(obj, dict) else [],
        "discard": obj.get("discard", []) if isinstance(obj, dict) else [],
        "inference_justifications": obj.get("inference_justifications", []) if isinstance(obj, dict) else [],
        "_model_raw_ok": isinstance(obj, dict) and bool(obj.get("findings")),
    }


def main(argv: list[str]) -> int:
    model = "llama3.1:8b"
    if "--model" in argv:
        model = argv[argv.index("--model") + 1]
    d = json.loads(DICT.read_text())
    cases = json.loads(CASES.read_text())["cases"]
    IR_DIR.mkdir(exist_ok=True)
    for c in cases:
        print(f"decompose[{model}]: {c['id']} ...", flush=True)
        try:
            raw = ollama(model, prompt_for(c["vignette"], d))
        except Exception as e:  # noqa: BLE001 - report and continue (BATCH-safe)
            print(f"  ERROR: {e}", file=sys.stderr)
            raw = "{}"
        ir = coerce_ir(c["id"], raw)
        (IR_DIR / f"{c['id']}.json").write_text(json.dumps(ir, indent=2) + "\n")
        print(f"  -> {len(ir['findings'])} findings, {len(ir['discard'])} discarded"
              + ("" if ir["_model_raw_ok"] else "  [model output not parseable]"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
