"""Run named copy-aware policy mutants in disposable local-module copies."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
REPO = PACKAGE.parents[3]
DEPENDENCY = REPO / "code/packages/go/ftl-page-map"
FIXTURES = (
    "ftl-erase-victim.json",
    "ftl-copy-aware-victim.json",
)

MUTANTS = (
    (
        "exclude an unexcluded first block",
        "if len(victimExcluded) != 0 && victimExcluded[int(block)] {",
        "if block == 0 || len(victimExcluded) != 0 && victimExcluded[int(block)] {",
        "TestCopyAwareUnexcludedFirstBlockRemainsCandidate",
        "unexcluded block zero must remain a candidate",
    ),
    (
        "count target free pages as outside destinations",
        "freeOutside := allowedFree - localFree",
        "freeOutside := allowedFree",
        "TestCopyAwareRequiresExternalSparePages",
        "target-block free pages cannot be counted as external copy destinations",
    ),
    (
        "reverse benefit-to-copy ratio",
        "stale*bestDenominator > bestStale*denominator",
        "stale*bestDenominator < bestStale*denominator",
        "TestCopyAwareRatioBeatsStaleCount",
        "copy-aware ratio must beat a larger stale count",
    ),
    (
        "reverse exact-ratio tie order",
        "block < best.Block",
        "block > best.Block",
        "TestCopyAwareEqualScoreAndStaleCountTieChooseLowestIndex",
        "equal ratio and stale count ties choose lowest block index",
    ),
    (
        "ignore independent victim fence",
        "if len(victimExcluded) != 0 && victimExcluded[int(block)] {",
        "if false {",
        "TestCopyAwareVictimFenceSkipsAnOtherwiseEligibleBlock",
        "victim fence should leave block one as candidate",
    ),
    (
        "ignore independent destination fence",
        "if slot == ftlpagemap.Free && (len(destinationExcluded) == 0 || !destinationExcluded[int(block)]) {",
        "if slot == ftlpagemap.Free && (len(destinationExcluded) == 0 || !destinationExcluded[int(block)] || destinationExcluded[int(block)]) {",
        "TestCopyAwareDestinationFenceRemovesCandidateCapacity",
        "excluded destination block still contributed free capacity",
    ),
    (
        "count candidate local free as copy capacity",
        "if len(destinationExcluded) == 0 || !destinationExcluded[int(block)] {\n\t\t\t\t\tlocalFree++\n\t\t\t\t}",
        "if false {\n\t\t\t\t\tlocalFree++\n\t\t\t\t}",
        "TestCopyAwareCandidateCannotUseOwnFreeSlotsAsDestinations",
        "candidate-local free page counted as external destination",
    ),
)


def run(go: str, package: Path, test_name: str = "") -> subprocess.CompletedProcess[str]:
    command = [go, "test", "-count=1"]
    if test_name:
        command += ["-run", f"^{test_name}$"]
    command.append("./...")
    return subprocess.run(command, cwd=package, text=True, capture_output=True, check=False)


def copy_go_module(source: Path, destination: Path) -> None:
    destination.mkdir(parents=True)
    for item in source.glob("*.go"):
        shutil.copy2(item, destination / item.name)
    shutil.copy2(source / "go.mod", destination / "go.mod")


def materialize(root: Path) -> Path:
    package = root / "code/packages/go/ftl-erase-victim"
    copy_go_module(PACKAGE, package)
    copy_go_module(DEPENDENCY, root / "code/packages/go/ftl-page-map")
    fixture_dir = root / "code/specs/fixtures/database-media-v1"
    fixture_dir.mkdir(parents=True)
    for filename in FIXTURES:
        shutil.copy2(REPO / "code/specs/fixtures/database-media-v1" / filename, fixture_dir / filename)
    return package


def main() -> int:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="ftl-copy-aware-mutants-") as temporary:
        root = Path(temporary)
        clean_package = materialize(root / "clean")
        clean = run(go, clean_package)
        if clean.returncode:
            print(clean.stdout, end="")
            print(clean.stderr, end="", file=os.sys.stderr)
            raise SystemExit("copy-aware clean control failed")
        print("clean control: pass")
        scalar_test = "TestCopyAwareVictimFixtureRejectsInvalidUnicodeScalars"
        scalar_clean = subprocess.run(
            [go, "test", "-count=1", "-v", "-run", f"^{scalar_test}$", "./..."],
            cwd=clean_package, text=True, capture_output=True, check=False,
        )
        if scalar_clean.returncode != 0 or f"--- PASS: {scalar_test}" not in scalar_clean.stdout:
            raise RuntimeError("copy-aware scalar clean witness did not pass by exact name")
        scalar_package = materialize(root / "scalar-bypass")
        scalar_source = scalar_package / "fixture_test.go"
        scalar_original = scalar_source.read_text(encoding="utf-8")
        scalar_anchor = "if err := validatePlanningSurrogateEscapes(raw); err != nil {\n\t\treturn err\n\t}"
        scalar_bypass = "if err := error(nil); err != nil {\n\t\treturn err\n\t}"
        if scalar_original.count(scalar_anchor) != 1:
            raise RuntimeError("copy-aware scalar bypass anchor is not unique")
        scalar_source.write_text(scalar_original.replace(scalar_anchor, scalar_bypass, 1), encoding="utf-8")
        scalar_result = subprocess.run(
            [go, "test", "-count=1", "-v", "-run", f"^{scalar_test}$", "./..."],
            cwd=scalar_package, text=True, capture_output=True, check=False,
        )
        scalar_output = scalar_result.stdout + scalar_result.stderr
        if scalar_result.returncode == 0 or f"--- FAIL: {scalar_test}" not in scalar_output or "fixture reader accepted an invalid Unicode scalar spelling" not in scalar_output:
            print(scalar_output, end="")
            raise SystemExit("copy-aware scalar bypass survived or missed its named witness")
        print(f"killed copy-aware-scalar-check-bypass: --- FAIL: {scalar_test}; assertion=fixture reader accepted an invalid Unicode scalar spelling")

        for index, (name, old, new, test_name, witness) in enumerate(MUTANTS):
            package = materialize(root / f"mutant-{index}")
            source = package / "victim.go"
            content = source.read_text(encoding="utf-8")
            section = content
            start = 0
            end = len(content)
            if section.count(old) != 1:
                raise SystemExit(f"{name}: scoped mutation anchor occurs {section.count(old)} times")
            content = content[:start] + section.replace(old, new, 1) + content[end:]
            source.write_text(content, encoding="utf-8")
            result = run(go, package, test_name)
            output = result.stdout + result.stderr
            if result.returncode == 0 or f"--- FAIL: {test_name}" not in output or witness not in output:
                print(output, end="")
                raise SystemExit(f"mutant failed to reach intended witness: {name}")
            print(f"killed {name}: {witness}")

    print(f"PASS clean control and {len(MUTANTS)} intended mutation witnesses")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
