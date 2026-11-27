"""Build pure OCaml/Swift libraries and replay the canonical test-only corpus."""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


FIXTURE_ROOT = Path(__file__).resolve().parent
REPOSITORY_ROOT = FIXTURE_ROOT.parents[3]
PACKAGE_ROOTS = {
    "ocaml": REPOSITORY_ROOT / "code/packages/ocaml/unix-history-simulator",
    "swift": REPOSITORY_ROOT / "code/packages/swift/unix-history-simulator",
}


def run(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    capture_output: bool = False,
) -> str:
    if capture_output:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
        )
        return result.stdout
    subprocess.run(command, cwd=cwd, env=env, check=True)
    return ""


def fixture_replay(language: str, command: str) -> int:
    executable = os.environ.get("HOS_FIXTURE_EXECUTABLE")
    if not executable:
        raise RuntimeError("fixture replay requires the build driver's temporary adapter path")
    result = subprocess.run(
        [executable, command],
        check=True,
        text=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
    )
    sys.stdout.write(result.stdout)
    return 0


def prepare_ocaml(package_root: Path, temporary_root: Path) -> Path:
    build_root = temporary_root / "ocaml-package"
    run(["dune", "build", "--build-dir", str(build_root), "src/unix_history_simulator.cma"], cwd=package_root)
    run(["dune", "runtest", "--build-dir", str(build_root)], cwd=package_root)

    archives = list(build_root.rglob("unix_history_simulator.cma"))
    if len(archives) != 1:
        raise RuntimeError(f"expected one OCaml bytecode library archive, found {archives}")
    adapter_source = FIXTURE_ROOT / "test-adapters/ocaml/fixture_cli.ml"
    adapter = temporary_root / ("ocaml-fixture-cli.exe" if os.name == "nt" else "ocaml-fixture-cli")
    run(
        [
            "ocamlc",
            "-I",
            str(archives[0].parent),
            str(archives[0]),
            str(adapter_source),
            "-o",
            str(adapter),
        ],
        cwd=package_root,
    )
    return adapter


def swift_command() -> list[str]:
    if platform.system() == "Darwin" and shutil.which("xcrun"):
        return ["xcrun", "swift"]
    return ["swift"]


def prepare_swift(package_root: Path, temporary_root: Path) -> Path:
    swift = swift_command()
    package_build = temporary_root / "swift-package"
    run([*swift, "test", "--scratch-path", str(package_build)], cwd=package_root)

    adapter_root = FIXTURE_ROOT / "test-adapters/swift"
    adapter_build = temporary_root / "swift-adapter"
    run(
        [*swift, "build", "--package-path", str(adapter_root), "--scratch-path", str(adapter_build)],
        cwd=package_root,
    )
    bin_path = Path(
        run(
            [
                *swift,
                "build",
                "--package-path",
                str(adapter_root),
                "--scratch-path",
                str(adapter_build),
                "--show-bin-path",
            ],
            cwd=package_root,
            capture_output=True,
        ).strip()
    )
    executable = bin_path / ("fixture-cli.exe" if os.name == "nt" else "fixture-cli")
    if not executable.is_file():
        raise RuntimeError(f"Swift fixture adapter was not produced at {executable}")
    return executable


def main() -> int:
    if len(sys.argv) >= 3 and sys.argv[2] == "--fixture":
        if len(sys.argv) != 4:
            raise SystemExit("usage: build_ocaml_swift.py LANGUAGE --fixture COMMAND")
        return fixture_replay(sys.argv[1], sys.argv[3])

    if len(sys.argv) != 2 or sys.argv[1] not in PACKAGE_ROOTS:
        raise SystemExit("usage: build_ocaml_swift.py ocaml|swift")

    language = sys.argv[1]
    package_root = PACKAGE_ROOTS[language].resolve()
    if Path.cwd().resolve() != package_root:
        raise RuntimeError(f"run from {package_root}; got {Path.cwd().resolve()}")

    with tempfile.TemporaryDirectory(prefix=f"historical-unix-{language}-") as temporary_name:
        temporary_root = Path(temporary_name)
        environment = os.environ.copy()
        environment.update(
            {
                "TMPDIR": str(temporary_root),
                "TMP": str(temporary_root),
                "TEMP": str(temporary_root),
                "SWIFTPM_MODULECACHE_OVERRIDE": str(temporary_root / "swift-module-cache"),
                "CLANG_MODULE_CACHE_PATH": str(temporary_root / "clang-module-cache"),
            }
        )
        adapter = (
            prepare_ocaml(package_root, temporary_root)
            if language == "ocaml"
            else prepare_swift(package_root, temporary_root)
        )
        environment["HOS_FIXTURE_EXECUTABLE"] = str(adapter)
        replay_output = run(
            [
                sys.executable,
                str(FIXTURE_ROOT / "replay_cli.py"),
                sys.executable,
                str(Path(__file__).resolve()),
                language,
                "--fixture",
            ],
            cwd=package_root,
            env=environment,
        )
        sys.stdout.write(replay_output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
