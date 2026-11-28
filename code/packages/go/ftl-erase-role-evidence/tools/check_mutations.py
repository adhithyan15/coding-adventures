"""Run clean-controlled defects against the role evidence package."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
MAP_PACKAGE = PACKAGE.parent / "ftl-page-map"

MUTATIONS = (
    (
        "skip-selected-sequence-check",
        "TestQualifyPrecedenceAndZeroResults",
        "erase intent sequence does not match",
        "if sequence == 0 || intent.Sequence != sequence {",
        "if false {",
    ),
    (
        "accept-programmed-unowned-page",
        "TestQualifyPrecedenceAndZeroResults",
        "programmed marker is not backed by stale intent",
        "if observed.Programmed[page] && !stale {",
        "if observed.Programmed[page] && !stale && false {",
    ),
)


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, check=False, text=True, capture_output=True)


def copy_go_package(source: Path, destination: Path) -> None:
    destination.mkdir(parents=True)
    for path in source.glob("*.go"):
        shutil.copy2(path, destination / path.name)
    shutil.copy2(source / "go.mod", destination / "go.mod")


def materialize(root: Path) -> Path:
    package = root / "code/packages/go/ftl-erase-role-evidence"
    copy_go_package(PACKAGE, package)
    copy_go_package(MAP_PACKAGE, root / "code/packages/go/ftl-page-map")
    return package


def replace_once(path: Path, old: str, new: str) -> None:
    source = path.read_text(encoding="utf-8")
    count = source.count(old)
    if count != 1:
        raise RuntimeError(f"{path.name}: found {count} mutation anchors, expected one")
    path.write_text(source.replace(old, new, 1), encoding="utf-8")


def main() -> int:
    go = os.environ.get("GO", "go")
    with tempfile.TemporaryDirectory(prefix="ftl-erase-role-evidence-mutations-") as temporary:
        root = Path(temporary)
        clean = run([go, "test", "-count=1", "./..."], materialize(root / "clean"))
        if clean.returncode:
            print(clean.stdout, end="")
            print(clean.stderr, end="", file=os.sys.stderr)
            raise RuntimeError("clean control failed")
        print("clean control: pass")

        for name, test_name, witness, old, replacement in MUTATIONS:
            package = materialize(root / name)
            replace_once(package / "role_evidence.go", old, replacement)
            result = run([go, "test", "-count=1", "-run", f"^{test_name}$", "."], package)
            output = result.stdout + result.stderr
            if result.returncode == 0 or f"--- FAIL: {test_name}" not in output or witness not in output:
                print(output, end="")
                raise RuntimeError(f"mutation lacked its intended witness: {name}")
            print(f"killed {name}: {witness}")

        package = materialize(root / "reuse-returned-buffer")
        source = package / "role_evidence.go"
        replace_once(
            source,
            "var (\n\tErrInvalidGeometry",
            "var (\n\troleBuffer []ftlpagemap.PhysicalStatus\n\tErrInvalidGeometry",
        )
        replace_once(
            source,
            "roles := make([]ftlpagemap.PhysicalStatus, pageCount)",
            "if len(roleBuffer) != pageCount { roleBuffer = make([]ftlpagemap.PhysicalStatus, pageCount) }\n\troles := roleBuffer",
        )
        result = run([go, "test", "-count=1", "-run", "^TestQualifyDetachedResultsAcrossCalls$", "."], package)
        output = result.stdout + result.stderr
        witness = "later call changed earlier returned roles"
        if result.returncode == 0 or "--- FAIL: TestQualifyDetachedResultsAcrossCalls" not in output or witness not in output:
            print(output, end="")
            raise RuntimeError("shared-result-buffer mutation lacked its intended witness")
        print(f"killed reuse-returned-buffer: {witness}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
