"""Run clean-controlled semantic mutations in disposable module copies."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOWER = ROOT.parent / "hdd-write-crash-cuts"

MUTATIONS = (
    (
        "persist-pending-data-early",
        "cache.go",
        "copy(next.pending[index][:], data)",
        "copy(next.pending[index][:], data)\n\tcopy(next.durable[index][:], data)",
        "TestWriteVisibilityOverwriteAndPowerLoss",
        "wrong overlay snapshot",
    ),
    (
        "retain-unrelated-pending-after-crash",
        "cache.go",
        "next = next.PowerLoss()",
		"next.pending[index] = [BytesPerSector]byte{}\n\t\tnext.dirty[index] = false",
        "TestFlushAcknowledgesOnlySelectedEntryAndCrashDropsAll",
        "unacknowledged flush did not drop all volatile writes",
    ),
    (
        "acknowledge-before-response",
        "cache.go",
        "Acknowledged: result.Receipt != nil",
		"Acknowledged: result.PersistedPrefixBytes == BytesPerSector",
        "TestEveryCutPreservesExactPrefixAndReceipt",
        "acknowledgement=true for cut",
    ),
    (
        "flushes-barrier-in-reverse-lba-order",
        "barrier.go",
        "for ordinal, lba := range dirtyLBAs {",
        "for ordinal := len(dirtyLBAs) - 1; ordinal >= 0; ordinal-- {\n\t\tlba := dirtyLBAs[ordinal]",
        "TestFlushDirtyMatchesIndependentArrayModelAcrossSubsetsAndCuts",
        "FlushDirty(",
    ),
    (
        "acknowledges-before-entire-dirty-set-is-durable",
        "barrier.go",
        "for ordinal, lba := range dirtyLBAs {",
        "if cut.Kind == BarrierComplete && len(dirtyLBAs) > 1 {\n\t\tdirtyLBAs = dirtyLBAs[:1]\n\t}\n\tfor ordinal, lba := range dirtyLBAs {",
        "TestFlushDirtyFullSetUsesAscendingLBARegardlessOfArrivalOrder",
        "FlushDirty(",
    ),
)


def run(go: str, cwd: Path, *args: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env.pop("PYTHONPATH", None)
    env.pop("PYTEST_ADDOPTS", None)
    env["GOTOOLCHAIN"] = "local"
    return subprocess.run(
        [go, *args], cwd=cwd, env=env, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=180,
        check=False,
    )


def main() -> int:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="hdd-writeback-mutations-") as raw:
        root = Path(raw) / "code/packages/go"
        package = root / "hdd-writeback-cache"
        lower = root / "hdd-write-crash-cuts"
        shutil.copytree(ROOT, package, ignore=shutil.ignore_patterns("bin", "obj", ".git"))
        shutil.copytree(LOWER, lower, ignore=shutil.ignore_patterns("bin", "obj", ".git"))
        for name, filename, old, new, test_name, expected in MUTATIONS:
            source = package / filename
            text = source.read_text()
            if text.count(old) != 1:
                raise RuntimeError(f"{name}: mutation anchor is not unique")
            baseline = run(go, package, "test", "-count=1", "-run", f"^{test_name}$", "./...")
            if baseline.returncode != 0:
                raise RuntimeError(f"{name}: clean baseline failed\n{baseline.stdout}")
            source.write_text(text.replace(old, new, 1))
            mutant = run(go, package, "test", "-count=1", "-run", f"^{test_name}$", "./...")
            if mutant.returncode == 0 or expected not in mutant.stdout:
                raise RuntimeError(
                    f"{name}: missing named assertion failure ({expected!r})\n{mutant.stdout}"
                )
            print(f"PASS {name}: clean test passed; intended assertion failed")
            source.write_text(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
