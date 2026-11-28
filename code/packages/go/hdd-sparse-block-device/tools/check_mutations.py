#!/usr/bin/env python3
"""Kill write-through and retained-pending HDD cache mutants."""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
REPOSITORY = PACKAGE.parents[3]
MODULE_PATH = Path("code/packages/go/hdd-sparse-block-device")
DEPENDENCIES = (
    "block-device-contract",
    "hdd-barrier-plan",
    "hdd-geometry",
    "hdd-seek-timing",
    "hdd-timed-media",
    "hdd-write-crash-cuts",
    "hdd-writeback-cache",
)

MUTANTS = (
    (
        "write-through-before-barrier",
        "TestWriteBlockRemainsVolatileBeforeFlush",
        "write changed durable media before Flush",
        (
            (
                'hddtimedmedia "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-timed-media"',
                'hddtimedmedia "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-timed-media"\n\thddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"',
            ),
            (
                "device.pending[block] = pending",
                "device.pending[block] = pending\n\t_, _ = device.medium.WriteSector(block, pending[:], hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged})",
            ),
        ),
    ),
    (
        "retain-pending-after-cut",
        "TestEveryBarrierCutMatchesVisibleDurableModel",
        "pending overlay got=",
        (("clear(device.pending)", "// mutant omitted pending clear"),),
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
    environment.update({
        "GOTOOLCHAIN": "local", "GOWORK": "off",
        "GOCACHE": str(cache / "build-cache"),
        "GOMODCACHE": str(cache / "module-cache"), "GOPROXY": "off",
    })
    return subprocess.run(
        [go, "test", "-count=1", "-v", "-run", f"^{test}$", "./..."],
        cwd=package, env=environment, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, check=False, timeout=120,
    )


def main() -> None:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="hdd-sparse-block-mutants-") as temporary:
        root = Path(temporary)
        clean = materialize(root / "clean")
        for test in ("TestWriteBlockRemainsVolatileBeforeFlush", "TestEveryBarrierCutMatchesVisibleDurableModel"):
            result = run(go, clean, root / "cache", test)
            if result.returncode or f"--- PASS: {test}" not in result.stdout:
                raise SystemExit(f"clean control {test} failed\n{result.stdout}")
            print(f"CLEAN PASS {test}")

        for name, test, witness, replacements in MUTANTS:
            package = materialize(root / name)
            source = package / "device.go"
            original = source.read_text(encoding="utf-8")
            mutant = original
            for anchor, replacement in replacements:
                if mutant.count(anchor) != 1:
                    raise SystemExit(f"{name}: expected one exact anchor {anchor!r}")
                mutant = mutant.replace(anchor, replacement, 1)
            source.write_text(mutant, encoding="utf-8")
            result = run(go, package, root / "cache", test)
            if result.returncode != 1 or f"--- FAIL: {test}" not in result.stdout or witness not in result.stdout:
                raise SystemExit(f"{name} was not killed by intended assertion\n{result.stdout}")
            print(f"MUTANT KILLED {name}: {witness}")
            source.write_text(original, encoding="utf-8")
            if source.read_text(encoding="utf-8") != original:
                raise SystemExit(f"{name}: source restoration failed")


if __name__ == "__main__":
    main()
