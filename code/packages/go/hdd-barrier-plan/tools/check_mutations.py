#!/usr/bin/env python3
"""Kill exact terminal-step and premature-acknowledgement mutants."""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
REPOSITORY = PACKAGE.parents[3]
MODULE_PATH = Path("code/packages/go/hdd-barrier-plan")
DEPENDENCIES = (
    "block-device-contract",
    "hdd-geometry",
    "hdd-seek-timing",
    "hdd-write-crash-cuts",
    "hdd-writeback-cache",
)
TEST_NAME = "TestAllSmallDirtyMasksAndEveryBarrierCutMatchModel"
MUTANTS = (
    (
        "visit-after-terminal-cut",
        "stepCount = int(cut.SectorIndex) + 1",
        "stepCount = len(dirty)",
        "dirty=",
    ),
    (
        "acknowledge-after-all-media",
        "case hddwritebackcache.AfterAllMedia:\n\t\treceipt.PriorCompletedSectors = len(dirty)",
        "case hddwritebackcache.AfterAllMedia:\n\t\treceipt.PriorCompletedSectors = len(dirty)\n\t\treceipt.Acknowledged = true",
        "cut=",
    ),
)


def materialize(destination: Path) -> Path:
    package = destination / MODULE_PATH
    package.mkdir(parents=True)
    for source in PACKAGE.glob("*.go"):
        shutil.copy2(source, package / source.name)
    shutil.copy2(PACKAGE / "go.mod", package / "go.mod")
    for dependency in DEPENDENCIES:
        source_dir = REPOSITORY / "code/packages/go" / dependency
        target_dir = destination / "code/packages/go" / dependency
        target_dir.mkdir(parents=True)
        for source in source_dir.glob("*.go"):
            shutil.copy2(source, target_dir / source.name)
        shutil.copy2(source_dir / "go.mod", target_dir / "go.mod")
    return package


def run(go: str, package: Path, cache: Path) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment.update({
        "GOTOOLCHAIN": "local", "GOWORK": "off",
        "GOCACHE": str(cache / "build-cache"),
        "GOMODCACHE": str(cache / "module-cache"), "GOPROXY": "off",
    })
    return subprocess.run(
        [go, "test", "-count=1", "-v", "-run", f"^{TEST_NAME}$", "./..."],
        cwd=package, env=environment, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, check=False, timeout=90,
    )


def main() -> None:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="hdd-barrier-plan-mutants-") as temporary:
        root = Path(temporary)
        baseline = materialize(root / "clean")
        clean = run(go, baseline, root / "cache")
        if clean.returncode or f"--- PASS: {TEST_NAME}" not in clean.stdout:
            raise SystemExit(f"clean named control failed\n{clean.stdout}")
        print(f"CLEAN PASS {TEST_NAME}")

        for name, anchor, replacement, witness in MUTANTS:
            package = materialize(root / name)
            source = package / "plan.go"
            original = source.read_text(encoding="utf-8")
            if original.count(anchor) != 1:
                raise SystemExit(f"{name}: expected one source anchor")
            source.write_text(original.replace(anchor, replacement, 1), encoding="utf-8")
            result = run(go, package, root / "cache")
            if result.returncode != 1 or f"--- FAIL: {TEST_NAME}" not in result.stdout or witness not in result.stdout:
                raise SystemExit(f"{name} not killed by intended assertion\n{result.stdout}")
            print(f"MUTANT KILLED {name}: {witness}")
            source.write_text(original, encoding="utf-8")
            if source.read_text(encoding="utf-8") != original:
                raise SystemExit(f"{name}: source restoration failed")


if __name__ == "__main__":
    main()
