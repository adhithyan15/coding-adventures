#!/usr/bin/env python3
"""Run two clean-controlled HDD adapter semantic mutations."""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
GO = shutil.which("go")
DEPENDENCIES = (
    "block-device-contract",
    "memory-block-device",
    "hdd-write-crash-cuts",
    "hdd-writeback-cache",
)
MUTATIONS = (
    (
        "flush-only-first-dirty-block",
        'device.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})',
        'device.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}})',
        "TestFlushPersistsEveryDirtyBlock",
    ),
    (
        "retain-volatile-bytes-on-restart",
        "return &HDD{cache: device.cache.PowerLoss()}, nil",
        "return &HDD{cache: device.cache}, nil",
        "TestRestartSeparatesDurableAndPendingBranches",
    ),
)


def run(command: list[str], cwd: Path, expected_exit: int) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    if result.returncode != expected_exit:
        raise SystemExit(
            f"unexpected exit {result.returncode}, expected {expected_exit}: {' '.join(command)}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def main() -> None:
    if GO is None:
        raise SystemExit("go executable not found")
    clean = run([GO, "test", "-count=1", "./..."], PACKAGE, 0)
    print("clean: PASS")
    print(clean.stdout.strip())
    for name, anchor, replacement, witness in MUTATIONS:
        with tempfile.TemporaryDirectory(prefix=f"db06b2b2-{name}-") as temp:
            root = Path(temp) / "code" / "packages" / "go"
            root.mkdir(parents=True)
            mirror = root / "hdd-block-device"
            shutil.copytree(PACKAGE, mirror, ignore=shutil.ignore_patterns("tools", "testdata"))
            for dependency in DEPENDENCIES:
                shutil.copytree(PACKAGE.parent / dependency, root / dependency)
            source = mirror / "device.go"
            contents = source.read_text(encoding="utf-8")
            if contents.count(anchor) != 1:
                raise SystemExit(f"{name}: expected exactly one source anchor")
            mutated = contents.replace(anchor, replacement, 1)
            if name == "flush-only-first-dirty-block":
                import_anchor = 'hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"'
                if mutated.count(import_anchor) != 1:
                    raise SystemExit(f"{name}: missing exact dependency import anchor")
                mutated = mutated.replace(
                    import_anchor,
                    import_anchor + '\n\thddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"',
                    1,
                )
            source.write_text(mutated, encoding="utf-8")
            killed = run([GO, "test", "-count=1", "-run", f"^{witness}$", "."], mirror, 1)
            if "FAIL" not in killed.stdout or witness not in killed.stdout:
                raise SystemExit(f"{name}: intended test did not fail\n{killed.stdout}\n{killed.stderr}")
            print(f"{name}: KILLED by {witness}")


if __name__ == "__main__":
    main()
