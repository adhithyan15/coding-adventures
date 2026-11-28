"""Run clean and named DB04d runtime mutations in disposable module copies."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
FIXTURE = PACKAGE.parents[2] / "specs/fixtures/database-media-v1/nand-block-erase-cuts.json"
SOURCE_NAME = "erase.go"

MUTATIONS = (
    (
        "cleared-suffix-marker",
        "TestAfterPrefixPreservesSuffixPagesAndMarkers",
        "after-prefix preserves suffix page markers",
        "\t\tresult.IncompleteErase = true",
        (
            "\t\tif cut.PrefixPages < len(result.Pages) {\n"
            "\t\t\tresult.Pages[cut.PrefixPages].Programmed = false\n"
            "\t\t}\n\t\tresult.IncompleteErase = true"
        ),
    ),
    (
        "partial-cut-increments-count",
        "TestPartialEraseKeepsCompletedEraseCount",
        "partial erase changed completed count",
        "\t\tresult.IncompleteErase = true",
        "\t\tresult.CompletedErases++\n\t\tresult.IncompleteErase = true",
    ),
    (
        "premature-acknowledgement",
        "TestAfterMediaErasesFullyWithoutAcknowledgement",
        "after-media has no protocol acknowledgement",
        "\t\tacknowledged = cut.Kind == Complete",
        "\t\tacknowledged = true",
    ),
)


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, check=False, text=True, capture_output=True)


def materialize(root: Path) -> Path:
    package = root / "code/packages/go/nand-block-erase-cuts"
    package.mkdir(parents=True)
    for source in PACKAGE.glob("*.go"):
        shutil.copy2(source, package / source.name)
    shutil.copy2(PACKAGE / "go.mod", package / "go.mod")
    fixture = root / "code/specs/fixtures/database-media-v1/nand-block-erase-cuts.json"
    fixture.parent.mkdir(parents=True)
    shutil.copy2(FIXTURE, fixture)
    return package


def mutate(package: Path, old: str, replacement: str) -> None:
    source = package / SOURCE_NAME
    content = source.read_text(encoding="utf-8")
    if content.count(old) != 1:
        raise RuntimeError(f"mutation anchor occurs {content.count(old)} times; expected exactly once")
    source.write_text(content.replace(old, replacement, 1), encoding="utf-8")


def main() -> int:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="db04d-erase-mutations-") as temporary:
        root = Path(temporary)
        clean_package = materialize(root / "clean")
        clean = run([go, "test", "-count=1", "./..."], clean_package)
        if clean.returncode != 0:
            print(clean.stdout, end="")
            print(clean.stderr, end="", file=os.sys.stderr)
            print("clean control failed", file=os.sys.stderr)
            return 1
        print("clean control: pass")

        for name, test_name, witness, old, replacement in MUTATIONS:
            package = materialize(root / name)
            mutate(package, old, replacement)
            result = run([go, "test", "-count=1", "-run", f"^{test_name}$", "."], package)
            combined = result.stdout + result.stderr
            if result.returncode == 0:
                raise RuntimeError(f"mutant unexpectedly passed: {name}")
            if f"--- FAIL: {test_name}" not in combined or witness not in combined:
                print(combined, end="")
                raise RuntimeError(f"mutant did not execute its named witness: {name}")
            print(f"killed {name}: {witness}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
