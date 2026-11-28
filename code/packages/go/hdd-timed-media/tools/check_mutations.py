#!/usr/bin/env python3
"""Kill exact HDD timed-media byte-suffix and lost-ack mutants."""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
REPOSITORY = PACKAGE.parents[3]
MODULE_PATH = Path("code/packages/go/hdd-timed-media")
DEPENDENCIES = (
    "block-device-contract",
    "hdd-geometry",
    "hdd-seek-timing",
    "hdd-write-crash-cuts",
)
MUTANTS = (
    (
        "overwrite-unwritten-suffix",
        "TestBoth512GeometriesMatchSteppedByteAndTickModel",
        "snapshot got=",
        "copy(sector[:persisted], data[:persisted])",
        "copy(sector[persisted:], data[persisted:])",
    ),
    (
        "acknowledge-response-loss",
        "TestBoth512GeometriesMatchSteppedByteAndTickModel",
        "write block=0 cut=",
        "Acknowledged: acknowledged",
        "Acknowledged: acknowledged || persisted == sectorBytes",
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


def run(go: str, package: Path, cache: Path, test: str) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment.update(
        {
            "GOTOOLCHAIN": "local",
            "GOWORK": "off",
            "GOCACHE": str(cache / "build-cache"),
            "GOMODCACHE": str(cache / "module-cache"),
            "GOPROXY": "off",
        }
    )
    return subprocess.run(
        [go, "test", "-count=1", "-v", "-run", f"^{test}$", "./..."],
        cwd=package,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
        timeout=90,
    )


def main() -> None:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="hdd-timed-media-mutants-") as temporary:
        root = Path(temporary)
        baseline = materialize(root / "clean")
        clean = run(go, baseline, root / "cache", "TestBoth512GeometriesMatchSteppedByteAndTickModel")
        if clean.returncode or "--- PASS: TestBoth512GeometriesMatchSteppedByteAndTickModel" not in clean.stdout:
            raise SystemExit(f"clean named control failed\n{clean.stdout}")
        print("CLEAN PASS TestBoth512GeometriesMatchSteppedByteAndTickModel")

        for name, test, witness, anchor, replacement in MUTANTS:
            package = materialize(root / name)
            source = package / "media.go"
            original = source.read_text(encoding="utf-8")
            if original.count(anchor) != 1:
                raise SystemExit(f"{name}: expected one exact source anchor")
            source.write_text(original.replace(anchor, replacement, 1), encoding="utf-8")
            mutant = run(go, package, root / "cache", test)
            if mutant.returncode != 1 or f"--- FAIL: {test}" not in mutant.stdout or witness not in mutant.stdout:
                raise SystemExit(f"{name}: mutant was not killed by its named witness\n{mutant.stdout}")
            print(f"MUTANT KILLED {name}: {witness}")
            source.write_text(original, encoding="utf-8")
            if source.read_text(encoding="utf-8") != original:
                raise SystemExit(f"{name}: source restoration failed")


if __name__ == "__main__":
    main()
