"""Run clean and named page-map correctness mutations in disposable copies."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
FIXTURES = [PACKAGE.parents[2] / "specs/fixtures/database-media-v1" / name for name in ("ftl-page-map.json", "ftl-page-reclamation.json")]

MUTATIONS = (
    (
        "invalidate-retains-mapping",
        "TestInvalidateCurrentRemovesOnlyExactOwnerAndRetainsStaleSlot",
        "invalidation snapshot=",
        "result.logicalMap[int(logical)] = noMapping",
        "result.logicalMap[int(logical)] = physical",
    ),
    (
        "invalidated-page-becomes-free",
        "TestInvalidateCurrentRemovesOnlyExactOwnerAndRetainsStaleSlot",
        "invalidation snapshot=",
        "result.physical[int(physical)] = Stale",
        "result.physical[int(physical)] = Free",
    ),
    (
        "pending-owner-can-be-invalidated",
        "TestInvalidateCurrentSignedValidationAndPrecedence",
        "pending InvalidateCurrent",
        "func (table Table) InvalidateCurrent(logical, physical int64) (Table, error) {\n\tif !table.configured() {\n\t\treturn table, ErrInvalidGeometry\n\t}\n\tif logical < 0 || logical >= table.logicalCount {\n\t\treturn table, ErrInvalidLogicalPage\n\t}\n\tif physical < 0 || physical >= table.physicalCount {\n\t\treturn table, ErrInvalidPhysicalPage\n\t}\n\tif table.pending != nil {",
        "func (table Table) InvalidateCurrent(logical, physical int64) (Table, error) {\n\tif !table.configured() {\n\t\treturn table, ErrInvalidGeometry\n\t}\n\tif logical < 0 || logical >= table.logicalCount {\n\t\treturn table, ErrInvalidLogicalPage\n\t}\n\tif physical < 0 || physical >= table.physicalCount {\n\t\treturn table, ErrInvalidPhysicalPage\n\t}\n\tif false {",
    ),
    (
        "publish-at-reservation",
        "TestReservationKeepsOldMappingUntilReadyAndInvalidatesItAtomically",
        "pending relocation published early",
        "result.physical[physical] = Reserved",
        "result.physical[physical] = Reserved\n\tresult.logicalMap[int(logical)] = int64(physical)",
    ),
    (
        "reuse-spent-page",
        "TestSpentPhysicalSlotIsNeverSelectedAgain",
        "spent physical page was reused before reclamation",
        "if status == Free && (len(excluded) == 0 || !excluded[index]) {",
        "if status == Free || status == Stale {",
    ),
    (
        "retain-old-live-slot",
        "TestReservationKeepsOldMappingUntilReadyAndInvalidatesItAtomically",
        "rewrite did not publish and stale old slot atomically",
        "result.physical[int(previous)] = Stale",
        "result.physical[int(previous)] = Live",
    ),
    (
        "ignore-allocation-fence",
        "TestBeginExcludingSkipsFencedFreePage",
        "fenced lowest free page was selected",
        "if status == Free && (len(excluded) == 0 || !excluded[index]) {",
        "if status == Free {",
    ),
    (
        "partially-release-before-live-page",
        "TestReleaseRefusalKeepsEligiblePrefixUntouched",
        "eligible prefix changed on refused extent",
        "return table, ErrIneligibleExtent",
        "result := table.clone()\n\t\t\tfor prefix := firstPhysical; prefix < index; prefix++ { result.physical[int(prefix)] = Free }\n\t\t\treturn result, ErrIneligibleExtent",
    ),
    (
        "discard-outside-reservation-on-release",
        "TestReleasePreservesOutsideReservationAndReusesLowestPage",
        "release did not preserve outside reservation",
        "result := table.clone()\n\tfor index := firstPhysical; index < end; index++ {",
        "result := table.clone()\n\tresult.pending = nil\n\tfor index := firstPhysical; index < end; index++ {",
    ),
)


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, check=False, text=True, capture_output=True)


def materialize(root: Path) -> Path:
    package = root / "code/packages/go/ftl-page-map"
    package.mkdir(parents=True)
    for source in PACKAGE.glob("*.go"):
        shutil.copy2(source, package / source.name)
    shutil.copy2(PACKAGE / "go.mod", package / "go.mod")
    for fixture_source in FIXTURES:
        fixture = root / "code/specs/fixtures/database-media-v1" / fixture_source.name
        fixture.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(fixture_source, fixture)
    return package


def apply_mutation(package: Path, old: str, new: str) -> None:
    source = package / "page_map.go"
    content = source.read_text(encoding="utf-8")
    if content.count(old) != 1:
        raise RuntimeError(f"mutation anchor occurs {content.count(old)} times; expected one")
    source.write_text(content.replace(old, new, 1), encoding="utf-8")


def main() -> int:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="ftl-page-map-mutations-") as temporary:
        root = Path(temporary)
        clean = run([go, "test", "-count=1", "./..."], materialize(root / "clean"))
        if clean.returncode != 0:
            print(clean.stdout, end="")
            print(clean.stderr, end="", file=os.sys.stderr)
            raise RuntimeError("clean control failed")
        print("clean control: pass")

        for name, test_name, witness, old, new in MUTATIONS:
            package = materialize(root / name)
            apply_mutation(package, old, new)
            result = run([go, "test", "-count=1", "-run", f"^{test_name}$", "."], package)
            output = result.stdout + result.stderr
            if result.returncode == 0:
                raise RuntimeError(f"mutant unexpectedly passed: {name}")
            if f"--- FAIL: {test_name}" not in output or witness not in output:
                print(output, end="")
                raise RuntimeError(f"mutant did not reach the named witness: {name}")
            print(f"killed {name}: {witness}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
