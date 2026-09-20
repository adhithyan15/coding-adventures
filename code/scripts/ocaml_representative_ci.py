#!/usr/bin/env python3
"""Validate and assist the closed OCAML07 representative-package CI contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import stat
import subprocess  # nosec B404
import sys
import tarfile
from collections import Counter
from collections.abc import Mapping, Sequence
from pathlib import Path, PurePosixPath
from typing import Any

import ocaml_toolchain_lock

MANIFEST_RELATIVE_PATH = Path(
    "code/specs/fixtures/ocaml-representative-ci-v1/manifest.json"
)
WORKFLOW_RELATIVE_PATH = Path(".github/workflows/build-ocaml-representative.yml")
EXPECTED_WORKFLOW_SHA256 = (
    "918b8d931c5b7640c781e2a3094c929f5ef8d074f54819267d3a86ee64520677"
)
WINDOWS_COVERAGE_PREFIX_BLOCK = (
    '          coverage_prefix="$PWD/bisect"\n'
    '          if test "$RUNNER_OS" = "Windows"; then\n'
    '            coverage_prefix="$(cygpath -w "$PWD/bisect")"\n'
    "          fi\n"
    '          BISECT_FILE="$coverage_prefix" opam exec -- dune runtest --force \\\n'
)

VERSIONS = {
    "ocaml": "5.2.1",
    "opam": "2.5.2",
    "dune": "3.17.2",
    "alcotest": "1.9.0",
    "bisect_ppx": "2.8.3",
    "ocamlformat": "0.27.0",
    "odoc": "3.0.0",
    "yojson": "3.0.0",
}
ACTIONS = {
    "checkout": "3d3c42e5aac5ba805825da76410c181273ba90b1",
    "setup_ocaml": "15d660006c1d3110d77c34b7faa3bddefe8b82f0",
    "upload_artifact": "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
}
OPAM_REPOSITORY_COMMIT = "ba8cc66eb9e5baae7ebc88cf77f4c488d63d87ff"
COVERAGE_MINIMUM_BASIS_POINTS = 9500
PACKAGES = (
    {
        "id": "logic-gates",
        "root": "code/packages/ocaml/logic-gates",
        "opam_name": "coding-adventures-logic-gates",
        "version": "0.1.0",
        "local_dependencies": [],
        "coverage_sources": ["src/coding_adventures_logic_gates.ml"],
    },
    {
        "id": "graph",
        "root": "code/packages/ocaml/graph",
        "opam_name": "coding-adventures-graph",
        "version": "0.1.0",
        "local_dependencies": [],
        "coverage_sources": ["src/coding_adventures_graph.ml"],
    },
    {
        "id": "directed-graph",
        "root": "code/packages/ocaml/directed-graph",
        "opam_name": "coding-adventures-directed-graph",
        "version": "0.1.0",
        "local_dependencies": ["graph"],
        "coverage_sources": ["src/coding_adventures_directed_graph.ml"],
    },
    {
        "id": "state-machine",
        "root": "code/packages/ocaml/state-machine",
        "opam_name": "coding-adventures-state-machine",
        "version": "0.1.0",
        "local_dependencies": ["graph", "directed-graph"],
        "coverage_sources": ["src/coding_adventures_state_machine.ml"],
    },
)
TARGETS = {
    "linux-x64": {
        "runner": "ubuntu-24.04",
        "runner_os": "Linux",
        "runner_arch": "X64",
        "windows_compiler": None,
        "build_file": "BUILD",
    },
    "macos-arm64": {
        "runner": "macos-14",
        "runner_os": "macOS",
        "runner_arch": "ARM64",
        "windows_compiler": None,
        "build_file": "BUILD",
    },
    "windows-x64": {
        "runner": "windows-2022",
        "runner_os": "Windows",
        "runner_arch": "X64",
        "windows_compiler": "mingw",
        "build_file": "BUILD_windows",
    },
}
TOP_LEVEL_KEYS = {
    "schema_version",
    "versions",
    "actions",
    "opam_repository_commit",
    "coverage_minimum_basis_points",
    "packages",
    "analyzer",
    "downstream",
    "targets",
    "artifact",
    "governed_trees",
    "workflow_sha256",
}
PACKAGE_KEYS = {
    "id",
    "root",
    "opam_name",
    "version",
    "local_dependencies",
    "coverage_sources",
}
TARGET_KEYS = {"runner", "runner_os", "runner_arch", "windows_compiler", "build_file"}
ANALYZER = {
    "root": "code/packages/ocaml/ca-capability-analyzer",
    "opam_name": "coding-adventures-capability-analyzer",
    "version": "0.1.0",
    "executable": "_build/default/bin/main.exe",
}
ARTIFACT = {
    "name_prefix": "ocaml07-representative",
    "retention_days": 7,
    "required_entries": [
        "coverage",
        "documentation",
        "source-archives",
        "analyzer",
        "installed-packages.txt",
        "downstream",
        "toolchain.txt",
    ],
}
HEX_64 = re.compile(r"^[0-9a-f]{64}$")
COVERAGE_LINE = re.compile(
    r"^\s*(\d{1,3})\.(\d{2})\s*%\s+(\d+)\s*/\s*(\d+)\s+(.+?\.ml)\s*$"
)
BANNED_ARCHIVE_PARTS = {".git", "_build", "__pycache__", ".pytest_cache"}
BANNED_WORKFLOW_TEXT = (
    "continue-on-error",
    "|| true",
    "secrets.",
    "contents: write",
    "pull-requests: write",
    "if: always()",
    "success()",
)


class ContractError(ValueError):
    """Raised when checked evidence violates OCAML07."""


def _mapping(value: object, context: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise ContractError(f"{context} must be an object")
    return value


def _sequence(value: object, context: str) -> Sequence[Any]:
    if not isinstance(value, list):
        raise ContractError(f"{context} must be an array")
    return value


def _require_keys(value: Mapping[str, Any], expected: set[str], context: str) -> None:
    if set(value) != expected:
        raise ContractError(
            f"{context} keys must be exactly {sorted(expected)}; got {sorted(value)}"
        )


def _safe_relative_path(value: object, context: str) -> str:
    if not isinstance(value, str) or not value or "\\" in value:
        raise ContractError(f"{context} must be a nonempty POSIX relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or "." in path.parts:
        raise ContractError(f"{context} escapes or is not normalized: {value!r}")
    if path.as_posix() != value:
        raise ContractError(f"{context} is not normalized: {value!r}")
    return value


def _require_regular_path(path: Path, root: Path, context: str) -> None:
    try:
        relative = path.resolve(strict=True).relative_to(root.resolve(strict=True))
    except (OSError, ValueError) as exc:
        raise ContractError(f"{context} is missing or outside the repository") from exc
    current = root.resolve(strict=True)
    for part in relative.parts:
        current = current / part
        if current.is_symlink():
            raise ContractError(f"{context} has a linked path component")
    if not path.is_file():
        raise ContractError(f"{context} must be a regular file")
    mode = path.stat().st_mode
    if not stat.S_ISREG(mode):
        raise ContractError(f"{context} must be a regular file")


def _sha256_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def _tree_sha256(root: Path) -> str:
    if not root.is_dir() or root.is_symlink():
        raise ContractError(f"governed tree is missing or linked: {root}")
    digest = hashlib.sha256()
    entries = sorted(
        (
            (path.relative_to(root).as_posix(), path)
            for path in root.rglob("*")
        ),
        key=lambda entry: entry[0].encode("utf-8"),
    )
    included = 0
    for relative, path in entries:
        if set(PurePosixPath(relative).parts) & BANNED_ARCHIVE_PARTS or relative.endswith(
            ".coverage"
        ):
            continue
        if path.is_symlink():
            raise ContractError(f"governed tree contains a linked path: {path}")
        if not path.is_file():
            continue
        content = path.read_bytes()
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(str(len(content)).encode("ascii"))
        digest.update(b"\0")
        digest.update(content)
        included += 1
    if included == 0:
        raise ContractError(f"governed tree is empty: {root}")
    return digest.hexdigest()


def validate_manifest_shape(document: object) -> None:
    manifest = _mapping(document, "manifest")
    _require_keys(manifest, TOP_LEVEL_KEYS, "top-level")
    if manifest["schema_version"] != 1:
        raise ContractError("schema_version must be 1")
    if manifest["versions"] != VERSIONS:
        raise ContractError("versions must equal the reviewed OCAML07 pins")
    if manifest["actions"] != ACTIONS:
        raise ContractError("actions must equal the reviewed commit pins")
    if manifest["opam_repository_commit"] != OPAM_REPOSITORY_COMMIT:
        raise ContractError("opam_repository_commit must equal OCAML03")
    if manifest["coverage_minimum_basis_points"] != COVERAGE_MINIMUM_BASIS_POINTS:
        raise ContractError("coverage minimum must be exactly 9500 basis points")

    packages = _sequence(manifest["packages"], "packages")
    if len(packages) != len(PACKAGES):
        raise ContractError("package order must contain exactly four entries")
    for index, (actual, expected) in enumerate(zip(packages, PACKAGES)):
        package = _mapping(actual, f"packages[{index}]")
        _require_keys(package, PACKAGE_KEYS, f"packages[{index}]")
        _safe_relative_path(package["root"], f"packages[{index}].root")
        for source in _sequence(
            package["coverage_sources"], f"packages[{index}].coverage_sources"
        ):
            _safe_relative_path(source, f"packages[{index}].coverage_sources")
        if package != expected:
            if package.get("local_dependencies") != expected["local_dependencies"]:
                raise ContractError(f"package {expected['id']} dependencies drifted")
            raise ContractError("package order and identities must equal OCAML07")

    analyzer = _mapping(manifest["analyzer"], "analyzer")
    if analyzer != ANALYZER:
        raise ContractError("analyzer must equal the reviewed OCAML06 package")
    for key in ("root", "executable"):
        _safe_relative_path(analyzer[key], f"analyzer.{key}")

    downstream = _mapping(manifest["downstream"], "downstream")
    _require_keys(downstream, {"root", "cases"}, "downstream")
    _safe_relative_path(downstream["root"], "downstream.root")
    cases = _mapping(downstream["cases"], "downstream.cases")
    if list(cases) != ["representative", "transitive-leaf"]:
        raise ContractError("downstream cases must be representative then transitive-leaf")
    for case_id, case_value in cases.items():
        case = _mapping(case_value, f"downstream.cases.{case_id}")
        _require_keys(case, {"expected_file", "expected_sha256"}, f"case {case_id}")
        _safe_relative_path(case["expected_file"], f"case {case_id}.expected_file")
        if not isinstance(case["expected_sha256"], str) or not HEX_64.fullmatch(
            case["expected_sha256"]
        ):
            raise ContractError(f"case {case_id} digest must be lowercase SHA-256")

    targets = _mapping(manifest["targets"], "targets")
    if targets != TARGETS:
        raise ContractError("targets must equal the exact OCAML03 platform matrix")
    for target_id, target in targets.items():
        _require_keys(_mapping(target, target_id), TARGET_KEYS, f"target {target_id}")

    if manifest["artifact"] != ARTIFACT:
        raise ContractError("artifact contract drifted")
    governed = _mapping(manifest["governed_trees"], "governed_trees")
    expected_roots = [package["root"] for package in PACKAGES] + [
        ANALYZER["root"],
        downstream["root"],
    ]
    if list(governed) != expected_roots:
        raise ContractError("governed_trees must list exact roots in dependency order")
    for root, digest in governed.items():
        _safe_relative_path(root, "governed tree root")
        if not isinstance(digest, str) or not HEX_64.fullmatch(digest):
            raise ContractError(f"governed tree {root} digest must be lowercase SHA-256")
    workflow_digest = manifest["workflow_sha256"]
    if not isinstance(workflow_digest, str) or not HEX_64.fullmatch(workflow_digest):
        raise ContractError("workflow_sha256 must be lowercase SHA-256")
    if workflow_digest != EXPECTED_WORKFLOW_SHA256:
        raise ContractError("workflow_sha256 differs from the validator identity")


def load_manifest(repo_root: Path) -> dict[str, Any]:
    path = repo_root / MANIFEST_RELATIVE_PATH
    _require_regular_path(path, repo_root, "manifest")
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise ContractError(f"cannot read OCAML07 manifest: {exc}") from exc
    validate_manifest_shape(document)
    return document


def validate_workflow_text(manifest: Mapping[str, Any], workflow_text: str) -> None:
    digest = _sha256_bytes(workflow_text.encode("utf-8"))
    if digest != manifest["workflow_sha256"] or digest != EXPECTED_WORKFLOW_SHA256:
        raise ContractError("workflow digest differs from the closed OCAML07 identity")
    validate_workflow_policy_text(workflow_text)


def validate_workflow_policy_text(workflow_text: str) -> None:
    """Validate the security-sensitive workflow surface independent of its digest."""

    for banned in BANNED_WORKFLOW_TEXT:
        if banned in workflow_text:
            raise ContractError(f"workflow contains forbidden text: {banned}")
    if workflow_text.count("${{ github.token }}") != 1:
        raise ContractError("workflow automatic token use must occur exactly once")
    if "persist-credentials: false" not in workflow_text:
        raise ContractError("workflow must disable checkout credential persistence")
    expected_actions = Counter(
        {
            f"actions/checkout@{ACTIONS['checkout']}": 2,
            f"ocaml/setup-ocaml@{ACTIONS['setup_ocaml']}": 1,
            f"actions/upload-artifact@{ACTIONS['upload_artifact']}": 1,
        }
    )
    actual_actions = Counter(
        re.findall(r"^\s*uses:\s*([^\s#]+)\s*$", workflow_text, re.MULTILINE)
    )
    if actual_actions != expected_actions:
        raise ContractError(
            "workflow action allowlist/count drifted; "
            f"expected={dict(expected_actions)}, actual={dict(actual_actions)}"
        )
    for required in (
        "name: OCaml representative packages",
        "permissions:\n  contents: read",
        "fail-fast: false",
        "python code/scripts/ocaml_representative_ci.py validate-repository",
        "cmd.exe /D /S /C",
        'sh -c "$command"',
        "env -u CLICOLOR_FORCE opam exec -- odoc --version",
        "opam show --color=never --field=installed-version yojson",
        'opam show --color=never --field=installed-version "$opam_name"',
        "opam lint --strict",
        "dune build @install @doc --profile release",
        "validate-coverage",
        "create-source-archive",
        "validate-downstream-output",
        "if-no-files-found: error",
        "retention-days: 7",
    ):
        if required not in workflow_text:
            raise ContractError(f"workflow omits required OCAML07 command: {required}")
    if workflow_text.count(WINDOWS_COVERAGE_PREFIX_BLOCK) != 1:
        raise ContractError("workflow Windows analyzer coverage path guard drifted")
    if workflow_text.count('switch="$(opam switch show --safe)"') != 3:
        raise ContractError("workflow must capture the local switch in all temporary steps")
    if workflow_text.count('export OPAMSWITCH="$switch"') != 3:
        raise ContractError("workflow must bind the local switch in all temporary steps")
    try:
        parsed = ocaml_toolchain_lock.parse_restricted_workflow_yaml(workflow_text)
    except Exception as exc:
        raise ContractError(f"workflow restricted YAML is invalid: {exc}") from exc
    if parsed.get("name") != "OCaml representative packages":
        raise ContractError("workflow name drifted")
    jobs = parsed.get("jobs")
    if not isinstance(jobs, Mapping) or set(jobs) != {"contract", "representative"}:
        raise ContractError("workflow jobs must be exactly contract and representative")


def validate_repository(repo_root: Path, *, check_workflow: bool = True) -> dict[str, Any]:
    manifest = load_manifest(repo_root)
    for root_text, expected_digest in manifest["governed_trees"].items():
        actual = _tree_sha256(repo_root / root_text)
        if actual != expected_digest:
            raise ContractError(f"governed tree digest mismatch: {root_text}")
    downstream_root = repo_root / manifest["downstream"]["root"]
    for case_id, case in manifest["downstream"]["cases"].items():
        path = downstream_root / case["expected_file"]
        _require_regular_path(path, repo_root, f"downstream {case_id}")
        if _sha256_bytes(path.read_bytes()) != case["expected_sha256"]:
            raise ContractError(f"downstream {case_id} digest mismatch")
    project = (downstream_root / "dune-project").read_text(encoding="utf-8")
    if "(implicit_transitive_deps false)" not in project:
        raise ContractError("downstream fixture must disable implicit transitive deps")
    for source_name in ("representative_smoke.ml", "transitive_leaf_smoke.ml"):
        source = (downstream_root / source_name).read_text(encoding="utf-8")
        if source.count("set_binary_mode_out stdout true;") != 1:
            raise ContractError(
                f"downstream fixture must make LF receipts portable: {source_name}"
            )
    if check_workflow:
        workflow = repo_root / WORKFLOW_RELATIVE_PATH
        _require_regular_path(workflow, repo_root, "workflow")
        validate_workflow_text(manifest, workflow.read_text(encoding="utf-8"))
    return manifest


def validate_coverage_summary(
    text: str, expected_sources: Sequence[str], minimum_basis_points: int
) -> dict[str, int]:
    found: dict[str, int] = {}
    for line in text.splitlines():
        match = COVERAGE_LINE.match(line)
        if not match:
            continue
        whole, fractional, covered, instrumented, source = match.groups()
        source = PurePosixPath(source.strip().replace("\\", "/")).as_posix()
        if source in found:
            raise ContractError(f"duplicate coverage source: {source}")
        if int(instrumented) <= 0:
            raise ContractError(f"coverage source has no instrumented lines: {source}")
        if int(covered) > int(instrumented):
            raise ContractError(f"coverage counts are invalid: {source}")
        found[source] = int(whole) * 100 + int(fractional)
    expected = list(expected_sources)
    missing = [source for source in expected if source not in found]
    if missing:
        raise ContractError(f"coverage summary is missing expected sources: {missing}")
    extras = sorted(set(found) - set(expected))
    if extras:
        raise ContractError(f"coverage summary contains unexpected sources: {extras}")
    for source, basis_points in found.items():
        if basis_points < minimum_basis_points:
            raise ContractError(
                f"coverage for {source} is below {minimum_basis_points / 100:.2f}%"
            )
    return found


def validate_archive_members(
    archive_path: Path, expected_root: str, expected_files: set[str]
) -> set[str]:
    seen: set[str] = set()
    try:
        with tarfile.open(archive_path, "r:gz") as archive:
            for member in archive.getmembers():
                path = PurePosixPath(member.name)
                if (
                    path.is_absolute()
                    or ".." in path.parts
                    or member.name != path.as_posix()
                ):
                    raise ContractError(f"unsafe source archive path: {member.name}")
                if not path.parts or path.parts[0] != expected_root:
                    raise ContractError("source archive entries must use one package root")
                relative = PurePosixPath(*path.parts[1:]).as_posix()
                if not relative or member.isdir():
                    continue
                if not member.isfile():
                    raise ContractError(
                        f"source archive entry is not regular: {member.name}"
                    )
                if relative in seen:
                    raise ContractError(f"duplicate source archive entry: {relative}")
                if set(PurePosixPath(relative).parts) & BANNED_ARCHIVE_PARTS or relative.endswith(
                    ".coverage"
                ):
                    raise ContractError(f"generated source archive entry: {relative}")
                seen.add(relative)
    except (OSError, tarfile.TarError) as exc:
        raise ContractError(f"cannot read source archive: {exc}") from exc
    if seen != expected_files:
        missing = sorted(expected_files - seen)
        extra = sorted(seen - expected_files)
        raise ContractError(f"source archive tracked-file mismatch; missing={missing}, extra={extra}")
    return seen


def _package(manifest: Mapping[str, Any], package_id: str) -> Mapping[str, Any]:
    for package in manifest["packages"]:
        if package["id"] == package_id:
            return package
    raise ContractError(f"unknown package id: {package_id}")


def _git_tracked_files(repo_root: Path, package_root: str) -> list[str]:
    command = ["git", "ls-files", "-z", "--", package_root]
    result = subprocess.run(  # nosec B603
        command,
        cwd=repo_root,
        check=True,
        capture_output=True,
    )
    prefix = f"{package_root}/"
    files = []
    for raw in result.stdout.split(b"\0"):
        if not raw:
            continue
        path = raw.decode("utf-8")
        if not path.startswith(prefix):
            raise ContractError(f"git returned path outside package root: {path}")
        files.append(path[len(prefix) :])
    if not files:
        raise ContractError(f"package has no tracked files: {package_root}")
    return sorted(files)


def create_source_archive(
    manifest: Mapping[str, Any], package_id: str, output: Path, repo_root: Path
) -> None:
    package = _package(manifest, package_id)
    tracked = _git_tracked_files(repo_root, package["root"])
    output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(output, "w:gz", format=tarfile.PAX_FORMAT) as archive:
        for relative in tracked:
            source = repo_root / package["root"] / relative
            _require_regular_path(source, repo_root, f"tracked package file {relative}")
            info = archive.gettarinfo(str(source), arcname=f"{package_id}/{relative}")
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            info.mtime = 0
            with source.open("rb") as handle:
                archive.addfile(info, handle)
    validate_archive_members(output, package_id, set(tracked))


def validate_source_archive(
    manifest: Mapping[str, Any], package_id: str, archive_path: Path, repo_root: Path
) -> None:
    package = _package(manifest, package_id)
    tracked = set(_git_tracked_files(repo_root, package["root"]))
    validate_archive_members(archive_path, package_id, tracked)


def validate_downstream_output(
    manifest: Mapping[str, Any], case_id: str, actual_path: Path, repo_root: Path
) -> None:
    cases = manifest["downstream"]["cases"]
    if case_id not in cases:
        raise ContractError(f"unknown downstream case: {case_id}")
    expected = repo_root / manifest["downstream"]["root"] / cases[case_id]["expected_file"]
    _require_regular_path(expected, repo_root, f"downstream expected {case_id}")
    try:
        actual = actual_path.read_bytes()
    except OSError as exc:
        raise ContractError(f"cannot read downstream output {actual_path}: {exc}") from exc
    if actual != expected.read_bytes():
        raise ContractError(f"downstream output differs byte-for-byte: {case_id}")


def validate_evidence(manifest: Mapping[str, Any], evidence_root: Path) -> None:
    for entry in manifest["artifact"]["required_entries"]:
        path = evidence_root / entry
        if not path.exists():
            raise ContractError(f"required evidence is missing: {entry}")
        if path.is_dir() and not any(child.is_file() for child in path.rglob("*")):
            raise ContractError(f"required evidence directory is empty: {entry}")
        if path.is_file() and path.stat().st_size == 0:
            raise ContractError(f"required evidence file is empty: {entry}")
    exact_files = [
        "installed-packages.txt",
        "toolchain.txt",
        "source-archives/SHA256SUMS",
        "downstream/representative.json",
        "downstream/transitive-leaf.json",
    ]
    for package in manifest["packages"]:
        package_id = package["id"]
        exact_files.extend(
            [
                f"coverage/{package_id}/coverage-summary.txt",
                f"documentation/{package_id}.log",
                f"analyzer/{package_id}.txt",
                (
                    "source-archives/"
                    f"coding-adventures-{package_id}-{package['version']}.tar.gz"
                ),
            ]
        )
        raw_coverage = list((evidence_root / "coverage" / package_id).glob("*.coverage"))
        if not raw_coverage or any(path.stat().st_size == 0 for path in raw_coverage):
            raise ContractError(f"raw coverage evidence is missing for {package_id}")
        documentation = evidence_root / "documentation" / package_id
        if not documentation.is_dir() or not any(
            path.is_file() and path.stat().st_size > 0
            for path in documentation.rglob("*.html")
        ):
            raise ContractError(f"HTML documentation evidence is missing for {package_id}")
    for relative in exact_files:
        path = evidence_root / relative
        if not path.is_file() or path.stat().st_size == 0:
            raise ContractError(f"required evidence file is missing or empty: {relative}")


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    repository = subparsers.add_parser("validate-repository")
    repository.add_argument("--repo-root", type=Path, default=Path.cwd())
    coverage = subparsers.add_parser("validate-coverage")
    coverage.add_argument("--repo-root", type=Path, default=Path.cwd())
    coverage.add_argument("--package")
    coverage.add_argument("--summary", type=Path)
    coverage.add_argument("--source", action="append", default=[])
    coverage.add_argument("--minimum", type=int, default=COVERAGE_MINIMUM_BASIS_POINTS)
    create = subparsers.add_parser("create-source-archive")
    create.add_argument("--repo-root", type=Path, default=Path.cwd())
    create.add_argument("--package", required=True)
    create.add_argument("--output", type=Path, required=True)
    archive = subparsers.add_parser("validate-source-archive")
    archive.add_argument("--repo-root", type=Path, default=Path.cwd())
    archive.add_argument("--package", required=True)
    archive.add_argument("--archive", type=Path, required=True)
    downstream = subparsers.add_parser("validate-downstream-output")
    downstream.add_argument("--repo-root", type=Path, default=Path.cwd())
    downstream.add_argument("--case", required=True)
    downstream.add_argument("--actual", type=Path, required=True)
    evidence = subparsers.add_parser("validate-evidence")
    evidence.add_argument("--repo-root", type=Path, default=Path.cwd())
    evidence.add_argument("--root", type=Path, required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.command == "validate-repository":
            validate_repository(args.repo_root.resolve())
        elif args.command == "validate-coverage":
            if args.package:
                manifest = load_manifest(args.repo_root.resolve())
                package = _package(manifest, args.package)
                sources = package["coverage_sources"]
                minimum = manifest["coverage_minimum_basis_points"]
            else:
                sources = args.source
                minimum = args.minimum
            if not args.summary:
                raise ContractError("coverage summary path is required")
            validate_coverage_summary(
                args.summary.read_text(encoding="utf-8"), sources, minimum
            )
        elif args.command == "create-source-archive":
            manifest = load_manifest(args.repo_root.resolve())
            create_source_archive(
                manifest, args.package, args.output, args.repo_root.resolve()
            )
        elif args.command == "validate-source-archive":
            manifest = load_manifest(args.repo_root.resolve())
            validate_source_archive(
                manifest, args.package, args.archive, args.repo_root.resolve()
            )
        elif args.command == "validate-downstream-output":
            manifest = load_manifest(args.repo_root.resolve())
            validate_downstream_output(
                manifest, args.case, args.actual, args.repo_root.resolve()
            )
        elif args.command == "validate-evidence":
            manifest = load_manifest(args.repo_root.resolve())
            validate_evidence(manifest, args.root)
        else:  # pragma: no cover
            raise ContractError(f"unknown command: {args.command}")
    except (ContractError, OSError, UnicodeError, subprocess.CalledProcessError) as exc:
        print(f"OCAML07 validation failed: {exc}", file=sys.stderr)
        return 1
    print(f"OCAML07 {args.command} passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
