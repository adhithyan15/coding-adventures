#!/usr/bin/env python3
"""test_cas.py - guard the MYCIN-2026 CAS: deterministic build + importable + decides.

Run:  python3 test_cas.py     (exit 0 = pass). CI runs the same.

Three checks:
  1. `cas_build.py --check` - the committed cas/objects/* match a fresh build
     (the content-addressed store is reproducible from lib/ + grounding/).
  2. Every object is importable: a case that `import`s the content-addressed root
     object pulls the whole grounded graph through the M3 import resolver.
  3. The grounded rulebook discriminates: a bacterial CSF picture -> bacterial
     leads; a viral picture -> viral leads. (Requires the adj-lang-cli binary; the
     decide check is skipped with a clear message if it is not built.)
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[3]  # .../coding-adventures

# A CAS hash is exactly 16 lowercase hex chars (sha256[:16]). Validate any hash
# read from the registry BEFORE it becomes a filesystem path or `.adj` content,
# so a tampered registry can't path-traverse or inject adj-lang statements.
HASH_RE = re.compile(r"\A[0-9a-f]{16}\Z")


def find_cli() -> Path | None:
    p = shutil.which("adj-lang-cli")
    if p:
        return Path(p)
    for prof in ("debug", "release"):
        cand = REPO / "code/packages/rust/target" / prof / "adj-lang-cli"
        if cand.exists():
            return cand
    return None


def check_build() -> None:
    r = subprocess.run([sys.executable, str(ROOT / "cas_build.py"), "--check"],
                       capture_output=True, text=True)
    assert r.returncode == 0, f"cas_build --check failed:\n{r.stdout}\n{r.stderr}"
    print("  [1/3] CAS is deterministic (--check passed)")


def decide(cli: Path, program: Path) -> dict:
    r = subprocess.run([str(cli), str(program)], capture_output=True, text=True)
    assert r.returncode == 0, f"adj-lang-cli exited {r.returncode}: {r.stderr}\n{r.stdout}"
    return json.loads(r.stdout)


def check_importable_and_decides(cli: Path) -> None:
    registry = json.loads((ROOT / "cas" / "registry.json").read_text())
    root_hash = registry["root"]
    assert root_hash and HASH_RE.match(root_hash), f"registry root not a valid hash: {root_hash!r}"
    assert (ROOT / "cas" / "objects" / f"{root_hash}.adj").exists()

    # A case must sit at cas/ level (its sandbox root) so `import "objects/<hash>.adj"`
    # stays in-sandbox - objects/ is a reachable subdir, no `../` escape.
    cas = ROOT / "cas"
    bacterial = cas / f"_test_bacterial_{os.getpid()}.adj"
    viral = cas / f"_test_viral_{os.getpid()}.adj"
    try:
        bacterial.write_text(
            f'import "objects/{root_hash}.adj"\n'
            "observe csf_gram_stain(positive)\n"
            "observe csf_neutrophilic_pleocytosis(high)\n"
            "observe csf_glucose(low)\n")
        viral.write_text(
            f'import "objects/{root_hash}.adj"\n'
            "observe csf_lymphocytic_pleocytosis(high)\n"
            "observe csf_glucose(normal)\n"
            "observe enteroviral_pcr(positive)\n")

        db = decide(cli, bacterial)
        assert db.get("decision", {}).get("leader") == "bacterial_meningitis", db
        print(f"  [2/3] case imports objects/{root_hash}.adj -> graph resolves")
        dv = decide(cli, viral)
        assert dv.get("decision", {}).get("leader") == "viral_meningitis", dv
        print("  [3/3] grounded rulebook discriminates (bacterial->bacterial, viral->viral)")
    finally:
        bacterial.unlink(missing_ok=True)
        viral.unlink(missing_ok=True)


def main() -> int:
    print("test_cas: guarding the MYCIN-2026 content-addressed library store")
    check_build()
    cli = find_cli()
    if cli is None:
        print("  [2-3/3] SKIPPED: adj-lang-cli not built "
              "(run `cargo build -p adj-lang-cli`). Build check still passed.")
        return 0
    check_importable_and_decides(cli)
    print("test_cas: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
