#!/usr/bin/env python3
"""Run exact clean controls for seek phase and portable clock admission."""
from __future__ import annotations

import hashlib
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "timing.go"
MUTANTS = (
    (
        "TestFixedDB02AllHeadPhaseBlocks",
        "fixed profile head=",
        "phase := addModulo(current.Tick%period, seek%period, period)",
        "phase := current.Tick % period",
    ),
    (
        "TestOneTickOverflowRefusesAtomically",
        "one-tick overflow=",
        "if rotation >= remainingAfterSeek {",
        "if rotation > remainingAfterSeek {",
    ),
)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run(name: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["go", "test", "-count=1", "-v", "-run", f"^{name}$", "./..."],
        cwd=ROOT,
        env=os.environ.copy(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )


def main() -> None:
    original = SOURCE.read_bytes()
    original_digest = digest(original)
    for name, witness, before, after in MUTANTS:
        clean = run(name)
        if clean.returncode or f"--- PASS: {name}" not in clean.stdout:
            raise SystemExit(f"clean named control did not pass: {name}\n{clean.stdout}")
        print(f"CLEAN PASS {name}")
        anchor = before.encode()
        if original.count(anchor) != 1:
            raise SystemExit(f"expected one mutation anchor for {name}")
        try:
            SOURCE.write_bytes(original.replace(anchor, after.encode(), 1))
            mutant = run(name)
            if (mutant.returncode == 0 or f"--- FAIL: {name}" not in mutant.stdout
                    or witness not in mutant.stdout):
                raise SystemExit(f"mutant survived or missed named witness: {name}\n{mutant.stdout}")
            print(f"MUTANT KILLED {name}")
        finally:
            SOURCE.write_bytes(original)
        if digest(SOURCE.read_bytes()) != original_digest:
            raise SystemExit("runtime source was not restored exactly")
    print(f"RESTORED SHA256 {original_digest}")


if __name__ == "__main__":
    main()
