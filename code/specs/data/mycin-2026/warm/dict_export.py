#!/usr/bin/env python3
"""dict_export.py - parse lib/meningitis-vocab.adj into dictionary.json.

MYCIN-2026 M6. The decomposer prompt needs the closed vocabulary (functors,
value domains, surface forms); `ir_to_adj.py` needs it to validate the IR. Rather
than maintain a second copy, we EXTRACT it from the authored `.adj` dictionary so
there is exactly one source of truth (the same file the rulebook `use`s and the
compiler enforces). Output: warm/dictionary.json.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VOCAB = ROOT / "lib" / "meningitis-vocab.adj"
OUT = ROOT / "warm" / "dictionary.json"

# A `define` block runs from one `define` to the next `define` or the closing `}`.
DEFINE_RE = re.compile(
    r"define\s+([a-z_]+)\s*:\s*(hypothesis|finding)([^}]*?)(?=\n\s*define\s|\n\s*\})",
    re.DOTALL,
)
VALUES_RE = re.compile(r"values\s*\[([^\]]*)\]")
STRING_RE = re.compile(r'"([^"]*)"')


def export() -> dict:
    src = VOCAB.read_text()
    findings = []
    hypotheses = []
    for m in DEFINE_RE.finditer(src):
        name, kind, body = m.group(1), m.group(2), m.group(3)
        # surfaces: every quoted string after the `surface` keyword in the body.
        surf_part = body.split("surface", 1)[1] if "surface" in body else ""
        surfaces = STRING_RE.findall(surf_part)
        if kind == "hypothesis":
            hypotheses.append({"name": name, "surfaces": surfaces})
        else:
            vm = VALUES_RE.search(body)
            values = [v.strip() for v in vm.group(1).split(",")] if vm else []
            findings.append({"functor": name, "value_domain": values, "surfaces": surfaces})
    return {
        "_doc": "Closed controlled vocabulary, extracted from lib/meningitis-vocab.adj "
                "(the single source of truth). The decomposer is constrained to these "
                "functors + value domains; ir_to_adj.py rejects any term outside them.",
        "domain": "bacterial_vs_viral_meningitis",
        "hypotheses": hypotheses,
        "findings": findings,
    }


def main() -> int:
    d = export()
    OUT.write_text(json.dumps(d, indent=2) + "\n")
    print(f"dict_export: {len(d['findings'])} findings, {len(d['hypotheses'])} hypotheses -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
