"""Kill three named evacuation-plan faults in isolated local-module copies."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
GO_ROOT = PACKAGE.parent
REPO_CODE = PACKAGE.parents[2]
FIXTURE = REPO_CODE / "specs/fixtures/database-media-v1/ftl-evacuation-plan.json"
DRAIN_FIXTURE = REPO_CODE / "specs/fixtures/database-media-v1/ftl-retirement-drain-plan.json"

MUTATIONS = (
    (
        "count-target-and-excluded-free-pages",
        "TestTargetAndExcludedFreeSlotsDoNotCount",
        "target or excluded free slots counted as spare capacity",
        "if candidateBlock == block || (len(excludedBlocks) != 0 && excludedBlocks[int(candidateBlock)]) {",
        "if len(excludedBlocks) != 0 && excludedBlocks[int(candidateBlock)] {",
    ),
    (
        "select-nonfree-destination",
        "TestDestinationsMustBeFree",
        "planner selected a non-free destination",
        "if status != ftlpagemap.Free {",
        "if status == ftlpagemap.Live {",
    ),
    (
        "return-partial-plan-on-shortage",
        "TestInsufficientCapacityReturnsZeroPlan",
        "insufficient capacity returned a partial plan",
        "if destinationIndex < len(plan.Copies) {\n\t\treturn Plan{}, ErrInsufficientCapacity\n\t}",
        "if destinationIndex < len(plan.Copies) {\n\t\tplan.Copies = plan.Copies[:destinationIndex]\n\t\treturn plan, ErrInsufficientCapacity\n\t}",
    ),
    (
        "require-stale-page-for-readable-drain",
        "TestCanonicalDrainFixtureModelFirstThenRuntime",
        "error=no_stale_page",
        "return enumerateCopies(snapshot, pagesPerBlock, block, excludedBlocks, false)",
        "return enumerateCopies(snapshot, pagesPerBlock, block, excludedBlocks, true)",
    ),
    (
        "plan-drain-into-fenced-block",
        "TestCanonicalDrainFixtureModelFirstThenRuntime",
        "all-live-draining-target-and-zero-erase-budget",
        "return enumerateCopies(snapshot, pagesPerBlock, block, excludedBlocks, false)",
        "return enumerateCopies(snapshot, pagesPerBlock, block, nil, false)",
    ),
    (
        "reverse-drain-source-order",
        "TestDrainOrderedCopiesAndLegacySharedResult",
        "all-live source order",
        "for physical := first; physical < first+pagesPerBlock; physical++ {",
        "for physical := first + pagesPerBlock - 1; physical >= first; physical-- {",
    ),
)


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, check=False, text=True, capture_output=True)


def copy_module(source: Path, destination: Path) -> None:
    destination.mkdir(parents=True)
    for path in source.glob("*.go"):
        shutil.copy2(path, destination / path.name)
    shutil.copy2(source / "go.mod", destination / "go.mod")


def materialize(root: Path) -> Path:
    package = root / "code/packages/go/ftl-evacuation-plan"
    copy_module(PACKAGE, package)
    copy_module(GO_ROOT / "ftl-page-map", root / "code/packages/go/ftl-page-map")
    fixture = root / "code/specs/fixtures/database-media-v1/ftl-evacuation-plan.json"
    fixture.parent.mkdir(parents=True)
    shutil.copy2(FIXTURE, fixture)
    drain_fixture = root / "code/specs/fixtures/database-media-v1/ftl-retirement-drain-plan.json"
    shutil.copy2(DRAIN_FIXTURE, drain_fixture)
    return package


def replace_once(path: Path, old: str, new: str) -> None:
    source = path.read_text(encoding="utf-8")
    count = source.count(old)
    if count != 1:
        raise RuntimeError(f"{path}: mutation anchor appears {count} times, expected once")
    path.write_text(source.replace(old, new, 1), encoding="utf-8")


def main() -> int:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="ftl-evacuation-plan-mutations-") as temporary:
        root = Path(temporary)
        clean_package = materialize(root / "clean")
        clean = run([go, "test", "-count=1", "./..."], clean_package)
        if clean.returncode != 0:
            print(clean.stdout, end="")
            print(clean.stderr, end="", file=sys.stderr)
            raise RuntimeError("clean control failed")
        print("clean control: pass")
        scalar_cases = (
            ("TestEvacuationFixtureRejectsInvalidUnicodeScalars", "fixture reader accepted an invalid Unicode scalar spelling"),
            ("TestDrainFixtureRejectsInvalidUnicodeScalars", "fixture reader accepted an invalid Unicode scalar spelling"),
        )
        scalar_anchor = "if err := validatePlanningSurrogateEscapes(raw); err != nil {\n\t\treturn err\n\t}"
        scalar_bypass = "if err := error(nil); err != nil {\n\t\treturn err\n\t}"
        for scalar_test, witness in scalar_cases:
            clean_scalar = run([go, "test", "-count=1", "-v", "-run", f"^{scalar_test}$", "./..."], clean_package)
            if clean_scalar.returncode != 0 or f"--- PASS: {scalar_test}" not in clean_scalar.stdout:
                print(clean_scalar.stdout, end="")
                print(clean_scalar.stderr, end="", file=sys.stderr)
                raise RuntimeError(f"clean scalar test did not pass by exact name: {scalar_test}")
            bypass_package = materialize(root / f"{scalar_test}-bypass")
            replace_once(bypass_package / "plan_test.go", scalar_anchor, scalar_bypass)
            bypass = run([go, "test", "-count=1", "-v", "-run", f"^{scalar_test}$", "./..."], bypass_package)
            output = bypass.stdout + bypass.stderr
            if bypass.returncode == 0 or f"--- FAIL: {scalar_test}" not in output or witness not in output:
                print(output, end="")
                raise RuntimeError(f"scalar bypass missed named witness: {scalar_test}")
            print(f"killed planning-scalar-check-bypass: --- FAIL: {scalar_test}; assertion={witness}")

        for name, test_name, witness, old, new in MUTATIONS:
            package = materialize(root / name)
            replace_once(package / "plan.go", old, new)
            result = run([go, "test", "-count=1", "-run", f"^{test_name}$", "."], package)
            output = result.stdout + result.stderr
            if result.returncode == 0:
                raise RuntimeError(f"mutant unexpectedly passed: {name}")
            if f"--- FAIL: {test_name}" not in output or witness not in output:
                print(output, end="")
                raise RuntimeError(f"mutant missed its named witness: {name}")
            print(f"killed {name}: {witness}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
