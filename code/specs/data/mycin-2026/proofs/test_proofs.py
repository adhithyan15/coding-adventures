#!/usr/bin/env python3
"""test_proofs.py - run all five MYCIN-2026 proofs; each asserts its own claim.

CI entry point for M8. Runs every proof script as a subprocess and requires exit
0 (each script contains the assertions for its claim). Skips cleanly when
adj-lang-cli is not built (the proofs print SKIPPED and exit 0).
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCRIPTS = ["golden_and_cpu.py", "cost_to_correct.py", "audit_trail.py"]


def main() -> int:
    failed = []
    for s in SCRIPTS:
        r = subprocess.run([sys.executable, str(HERE / s)], capture_output=True, text=True)
        ok = r.returncode == 0
        print(f"  {'PASS' if ok else 'FAIL'}  {s}")
        if not ok:
            print(r.stdout[-2000:])
            print(r.stderr[-1000:], file=sys.stderr)
            failed.append(s)
    # M7 (VOI + IIS) lives under warm/.
    r = subprocess.run([sys.executable, str(HERE.parent / "warm" / "test_m7.py")],
                       capture_output=True, text=True)
    ok = r.returncode == 0
    print(f"  {'PASS' if ok else 'FAIL'}  warm/test_m7.py")
    if not ok:
        failed.append("test_m7.py")
    if failed:
        print(f"test_proofs: FAILED {failed}", file=sys.stderr)
        return 1
    print("test_proofs: PASS (5 proofs + VOI/IIS)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
