#!/usr/bin/env python3
"""Build the blinded judge context files from a cross-domain run.

For each domain: produce the FRAMEWORK report by running run.py over its
{ingest, derived, spidered} (the byte-provenance trail + verdict/abstention), and
the PLAIN-CLAUDE report from its answer. Blind them as OUTPUT A / OUTPUT B
(framework alternates A/B by domain index) and write judge-<domain>.json with the
held-aside ground truth.

Run: python make_judge.py <crossdomain-results.json>
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _safe_domain(domain: str) -> str:
    """`domain` comes from agent-generated JSON and is interpolated into output
    filenames — validate it to a safe charset so it can never escape the directory
    (no `..`, `/`, NUL). Defense-in-depth against an arbitrary-file-write primitive."""
    if not re.fullmatch(r"[a-z0-9_-]+", domain):
        raise ValueError(f"unsafe domain id: {domain!r}")
    return domain


def framework_report(domain: str, rec: dict) -> str:
    fr = {"ingest": rec["ingest"], "derived": rec["derived"], "spidered": rec["spidered"]}
    p = HERE / f"_fr-{domain}.json"
    p.write_text(json.dumps(fr))
    out = subprocess.run(["python3", str(HERE / "run.py"), str(p)],
                         capture_output=True, text=True, env={**__import__("os").environ})
    text = out.stdout.strip() or out.stderr.strip()
    # STRIP the held-aside ground-truth line — run.py prints it for operator inspection,
    # but it must NOT reach the blind judge inside the framework's own report.
    return "\n".join(ln for ln in text.splitlines() if "ground truth (held aside)" not in ln)


def plain_report(rec: dict) -> str:
    pl = rec["plain"]
    return (f"ANSWER: {pl.get('answer','')}\n\nCONFIDENCE: {pl.get('confidence','')}\n\n"
            f"REASONING: {pl.get('reasoning','')}")


def main() -> None:
    res = json.loads(Path(sys.argv[1]).read_text())
    res = res.get("result", res)
    for i, rec in enumerate(res["per_domain"]):
        domain = _safe_domain(rec["domain"])
        fr = framework_report(domain, rec)
        pl = plain_report(rec)
        framework_is = "A" if i % 2 == 0 else "B"   # deterministic blinding, alternating
        a, b = (fr, pl) if framework_is == "A" else (pl, fr)
        ctx = {
            "domain": domain,
            "case_text": rec["ingest"]["case_text"],
            "ground_truth": rec["ingest"]["ground_truth"],
            "output_a": a, "output_b": b,
            "_framework_is": framework_is,   # for de-blinding after the judge returns
            "_framework_leading": rec["derived"].get("leading_diagnosis"),
            "_plain_answer": rec["plain"].get("answer"),
        }
        (HERE / f"judge-{domain}.json").write_text(json.dumps(ctx, indent=2))
        print(f"=== {domain} (framework={framework_is}) ===")
        print(f"  ground truth : {rec['ingest']['ground_truth'][:120]}")
        print(f"  framework    : leading={rec['derived'].get('leading_diagnosis')}")
        print(f"  plain Claude : {rec['plain'].get('answer','')[:90]}")


if __name__ == "__main__":
    main()
