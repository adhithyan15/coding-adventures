"""Compile the clean selector and prove four policy mutants are caught."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parent.parent
REPO = PACKAGE.parents[3]
DEPENDENCY = REPO / "code/packages/go/ftl-page-map"

MUTANTS = {
    "live block admitted": (
        "case ftlpagemap.Live:\n\t\t\t\tblocked = true",
        "case ftlpagemap.Live:\n\t\t\t\t// mutant: live page ignored",
        "TestLiveBlockCannotBeVictim",
        "block with a live page must be ineligible",
    ),
    "reserved block admitted": (
        "case ftlpagemap.Reserved:\n\t\t\t\tblocked = true",
        "case ftlpagemap.Reserved:\n\t\t\t\t// mutant: reservation ignored",
        "TestReservedBlockCannotBeVictim",
        "block with a reserved page must be ineligible",
    ),
    "free slots score as stale": (
        "case ftlpagemap.Stale:",
        "case ftlpagemap.Stale, ftlpagemap.Free:",
        "TestFreeSlotsDoNotEarnVictimScore",
        "free slots do not earn victim score",
    ),
    "reverse tie break": (
        "staleCount > best.StalePages",
        "staleCount >= best.StalePages",
        "TestTiesChooseLowestBlockIndex",
        "equal stale scores must choose the lowest block index",
    ),
}


def run(go: str, root: Path, test: str = "") -> subprocess.CompletedProcess[str]:
    args = [go, "test", "-count=1"]
    if test:
        args += ["-run", f"^{test}$"]
    args.append("./...")
    return subprocess.run(args, cwd=root, text=True, capture_output=True, check=False)


def main() -> int:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="ftl-erase-victim-mutants-") as temporary:
        base = Path(temporary) / "code/packages/go"
        package = base / "ftl-erase-victim"
        dependency = base / "ftl-page-map"
        shutil.copytree(PACKAGE, package, ignore=shutil.ignore_patterns("tools", "README.md", "CHANGELOG.md", "BUILD*"))
        shutil.copytree(DEPENDENCY, dependency, ignore=shutil.ignore_patterns("tools", "README.md", "CHANGELOG.md", "BUILD*"))
        fixture_dir = Path(temporary) / "code/specs/fixtures/database-media-v1"
        fixture_dir.mkdir(parents=True)
        for filename in ("ftl-erase-victim.json", "ftl-copy-aware-victim.json"):
            shutil.copy2(REPO / "code/specs/fixtures/database-media-v1" / filename, fixture_dir / filename)
        clean = run(go, package)
        if clean.returncode != 0:
            print(clean.stdout, clean.stderr)
            return 1
        print("clean control passed")
        scalar_test = "TestVictimFixtureRejectsInvalidUnicodeScalars"
        scalar_clean = subprocess.run(
            [go, "test", "-count=1", "-v", "-run", f"^{scalar_test}$", "./..."],
            cwd=package, text=True, capture_output=True, check=False,
        )
        if scalar_clean.returncode != 0 or f"--- PASS: {scalar_test}" not in scalar_clean.stdout:
            raise RuntimeError("erase-victim scalar clean witness did not pass by exact name")
        scalar_source = package / "fixture_test.go"
        scalar_original = scalar_source.read_text(encoding="utf-8")
        scalar_anchor = "if err := validatePlanningSurrogateEscapes(raw); err != nil {\n\t\treturn err\n\t}"
        scalar_bypass = "if err := error(nil); err != nil {\n\t\treturn err\n\t}"
        if scalar_original.count(scalar_anchor) != 1:
            raise RuntimeError("erase-victim scalar bypass anchor is not unique")
        scalar_source.write_text(scalar_original.replace(scalar_anchor, scalar_bypass, 1), encoding="utf-8")
        scalar_result = subprocess.run(
            [go, "test", "-count=1", "-v", "-run", f"^{scalar_test}$", "./..."],
            cwd=package, text=True, capture_output=True, check=False,
        )
        scalar_source.write_text(scalar_original, encoding="utf-8")
        scalar_output = scalar_result.stdout + scalar_result.stderr
        if scalar_result.returncode == 0 or f"--- FAIL: {scalar_test}" not in scalar_output or "fixture reader accepted an invalid Unicode scalar spelling" not in scalar_output:
            print(scalar_output)
            raise RuntimeError("erase-victim scalar bypass survived or missed its named witness")
        print(f"killed erase-victim-scalar-check-bypass: --- FAIL: {scalar_test}; assertion=fixture reader accepted an invalid Unicode scalar spelling")
        source_path = package / "victim.go"
        original = source_path.read_text()
        for label, (needle, replacement, test_name, witness) in MUTANTS.items():
            if needle not in original:
                raise RuntimeError(f"mutation anchor missing: {label}")
            source_path.write_text(original.replace(needle, replacement, 1))
            result = run(go, package, test_name)
            source_path.write_text(original)
            output = result.stdout + result.stderr
            if result.returncode == 0 or f"--- FAIL: {test_name}" not in output or witness not in output:
                print(f"mutant survived or failed for another reason: {label}\n{output}")
                return 1
            print(f"killed {label}: {test_name} / {witness}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
