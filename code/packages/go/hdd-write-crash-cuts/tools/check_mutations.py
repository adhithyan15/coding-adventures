"""Run a clean control and seven named HDD crash-cut mutants."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
REPOSITORY = PACKAGE.parents[3]
FIXTURE = REPOSITORY / "code/specs/fixtures/database-media-v1/hdd-write-crash-cuts.json"
MODULE_PATH = Path("code/packages/go/hdd-write-crash-cuts")
FIXTURE_PATH = Path("code/specs/fixtures/database-media-v1/hdd-write-crash-cuts.json")
MUTANTS = (
    (
        "write-no-prefix",
        "TestExhaustiveCutSweep",
        "prefix splice mismatch",
        "crash_cuts.go",
        "copy(image[int(lba)][:persisted], data[:persisted])",
        "copy(image[int(lba)][:0], data[:0])",
    ),
    (
        "write-one-extra-prefix-byte",
        "TestExhaustiveCutSweep",
        "prefix splice mismatch",
        "crash_cuts.go",
        "copy(image[int(lba)][:persisted], data[:persisted])",
        "copy(image[int(lba)][:persisted+1], data[:persisted+1])",
    ),
    (
        "reverse-prefix-and-suffix",
        "TestExhaustiveCutSweep",
        "prefix splice mismatch",
        "crash_cuts.go",
        "copy(image[int(lba)][:persisted], data[:persisted])",
        "copy(image[int(lba)][BytesPerSector-persisted:], data[BytesPerSector-persisted:])",
    ),
    (
        "change-a-neighbor-sector",
        "TestExhaustiveCutSweep",
        "sibling sector changed",
        "crash_cuts.go",
        "copy(image[int(lba)][:persisted], data[:persisted])",
        "copy(image[int(lba)][:persisted], data[:persisted])\n\timage[(int(lba)+1)%SectorCount][0] ^= 0xff",
    ),
    (
        "return-receipt-for-crash-cut",
        "TestExhaustiveCutSweep",
        "crash cut returned receipt",
        "crash_cuts.go",
        "if acknowledged {",
        "if acknowledged || cut.Kind != Acknowledged {",
    ),
    (
        "omit-acknowledgement-receipt",
        "TestExhaustiveCutSweep",
        "acknowledged cut omitted receipt",
        "crash_cuts.go",
        "if acknowledged {",
        "if false && acknowledged {",
    ),
    (
        "infer-cut-from-equal-bytes",
        "TestExhaustiveCutSweep",
        "equal old and requested bytes lost cut evidence",
        "crash_cuts.go",
        "persisted, acknowledged, err := validateCut(cut)",
        "persisted, acknowledged, err := validateCut(cut)\n\tif cut.Kind == DuringMediaWrite && string(before[int(lba)]) == string(data) {\n\t\tpersisted = 0\n\t}",
    ),
)


def run(go: str, cwd: Path, cache: Path, test: str | None = None) -> subprocess.CompletedProcess[str]:
    command = [go, "test", "-count=1"]
    if test:
        command.extend(("-run", f"^{test}$"))
    command.append("./...")
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
        command,
        cwd=cwd,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
        timeout=60,
    )


def materialize(destination: Path) -> Path:
    package = destination / MODULE_PATH
    package.mkdir(parents=True)
    for source in PACKAGE.glob("*.go"):
        shutil.copy2(source, package / source.name)
    shutil.copy2(PACKAGE / "go.mod", package / "go.mod")
    fixture = destination / FIXTURE_PATH
    fixture.parent.mkdir(parents=True)
    shutil.copy2(FIXTURE, fixture)
    return package


def mutate(source: Path, old: str, replacement: str) -> None:
    content = source.read_text(encoding="utf-8")
    if content.count(old) != 1:
        raise RuntimeError(f"expected one exact mutation anchor, found {content.count(old)}: {old!r}")
    source.write_text(content.replace(old, replacement, 1), encoding="utf-8")


def main() -> int:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="hdd-write-crash-cuts-mutations-") as temporary:
        root = Path(temporary)
        clean_package = materialize(root / "clean")
        baseline = run(go, clean_package, root / "cache")
        if baseline.returncode != 0:
            print(baseline.stdout, end="")
            print(baseline.stderr, end="", file=os.sys.stderr)
            raise RuntimeError("clean control failed")
        print("clean control passed")

        for name, test, witness, relative, old, replacement in MUTANTS:
            package = materialize(root / name)
            mutate(package / relative, old, replacement)
            result = run(go, package, root / "cache", test)
            output = result.stdout + result.stderr
            if result.returncode != 1 or f"--- FAIL: {test}" not in output or witness not in output:
                print(output, end="")
                raise RuntimeError(f"mutant failed outside its exact witness: {name}")
            print(f"killed {name}: {witness}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
