#!/usr/bin/env python3
"""Validate the checked OCaml CI toolchain and transitive solver evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path, PurePosixPath
from typing import Any

MANIFEST_RELATIVE_PATH = Path("code/specs/fixtures/ocaml-toolchain/toolchain-lock.json")
FIXTURE_ROOT_RELATIVE_PATH = Path("code/specs/fixtures/ocaml-toolchain")
SCAFFOLD_ROOT_RELATIVE_PATH = Path("code/specs/fixtures/scaffold-generator")
WORKFLOW_RELATIVE_PATH = Path(".github/workflows/build-ocaml.yml")

DIRECT_VERSIONS = {
    "ocaml": "5.2.1",
    "opam": "2.5.2",
    "dune": "3.17.2",
    "alcotest": "1.9.0",
    "bisect_ppx": "2.8.3",
    "ocamlformat": "0.27.0",
}
ACTIONS = {
    "checkout": "11d5960a326750d5838078e36cf38b85af677262",
    "setup_ocaml": "15d660006c1d3110d77c34b7faa3bddefe8b82f0",
}
OPAM_REPOSITORY_COMMIT = "ba8cc66eb9e5baae7ebc88cf77f4c488d63d87ff"
TARGETS = {
    "linux-x64": {
        "runner": "ubuntu-24.04",
        "runner_os": "Linux",
        "runner_arch": "X64",
        "windows_compiler": None,
    },
    "macos-x64": {
        "runner": "macos-14",
        "runner_os": "macOS",
        "runner_arch": "X64",
        "windows_compiler": None,
    },
    "windows-x64": {
        "runner": "windows-2022",
        "runner_os": "Windows",
        "runner_arch": "X64",
        "windows_compiler": "mingw",
    },
}

TOP_LEVEL_KEYS = {
    "schema_version",
    "direct_versions",
    "actions",
    "opam_repository_commit",
    "fixture_input_sha256",
    "targets",
}
TARGET_KEYS = {
    "runner",
    "runner_os",
    "runner_arch",
    "windows_compiler",
    "lock_file",
    "lock_sha256",
    "receipt_file",
    "receipt_sha256",
    "runner_image",
}
RUNNER_IMAGE_KEYS = {"image_os", "image_version"}
HEX_40 = re.compile(r"^[0-9a-f]{40}$")
HEX_64 = re.compile(r"^[0-9a-f]{64}$")
SEMVER = re.compile(r"^\d+\.\d+\.\d+$")
USES_REFERENCE = re.compile(r"^\s*uses:\s*([^\s#]+)\s*$", re.MULTILINE)


class ContractError(ValueError):
    """Raised when toolchain evidence violates OCAML03."""


def _require_closed_mapping(
    value: object,
    expected_keys: set[str],
    label: str,
) -> Mapping[str, Any]:
    if not isinstance(value, dict):
        raise ContractError(f"{label} must be an object")
    keys = set(value)
    missing = expected_keys - keys
    unknown = keys - expected_keys
    if missing or unknown:
        details = []
        if missing:
            details.append(f"missing keys {sorted(missing)}")
        if unknown:
            details.append(f"unknown keys {sorted(unknown)}")
        raise ContractError(f"{label} has {' and '.join(details)}")
    return value


def _require_commit(value: object, label: str) -> str:
    if not isinstance(value, str) or HEX_40.fullmatch(value) is None:
        raise ContractError(f"{label} must be 40 lowercase hexadecimal characters")
    return value


def _require_digest(value: object, label: str) -> str:
    if not isinstance(value, str) or HEX_64.fullmatch(value) is None:
        raise ContractError(f"{label} must be a lowercase SHA-256 digest")
    return value


def _safe_relative_path(value: object, label: str) -> PurePosixPath:
    if not isinstance(value, str) or not value or "\\" in value:
        raise ContractError(f"{label} must be a safe relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ContractError(f"{label} must be a safe relative path")
    return path


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_manifest(repo_root: Path) -> dict[str, Any]:
    """Load, parse, and shape-check the OCAML03 manifest."""

    path = repo_root / MANIFEST_RELATIVE_PATH
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ContractError(f"missing manifest: {path}") from exc
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise ContractError(f"cannot read manifest {path}: {exc}") from exc
    validate_manifest_shape(document)
    return document


def validate_manifest_shape(document: object) -> None:
    """Reject any open, malformed, or version-drifting manifest."""

    manifest = _require_closed_mapping(document, TOP_LEVEL_KEYS, "manifest")
    if manifest["schema_version"] != 1:
        raise ContractError("schema_version must equal 1")

    versions = _require_closed_mapping(
        manifest["direct_versions"], set(DIRECT_VERSIONS), "direct_versions"
    )
    for name, expected in DIRECT_VERSIONS.items():
        value = versions[name]
        if not isinstance(value, str) or SEMVER.fullmatch(value) is None:
            raise ContractError(f"direct_versions.{name} must be a semantic version")
        if value != expected:
            raise ContractError(
                f"direct_versions.{name} must equal reviewed version {expected}"
            )

    actions = _require_closed_mapping(manifest["actions"], set(ACTIONS), "actions")
    for name, expected in ACTIONS.items():
        value = _require_commit(actions[name], f"actions.{name}")
        if value != expected:
            raise ContractError(f"actions.{name} must equal reviewed commit {expected}")

    repository_commit = _require_commit(
        manifest["opam_repository_commit"], "opam_repository_commit"
    )
    if repository_commit != OPAM_REPOSITORY_COMMIT:
        raise ContractError(
            "opam_repository_commit must equal reviewed commit "
            f"{OPAM_REPOSITORY_COMMIT}"
        )
    _require_digest(manifest["fixture_input_sha256"], "fixture_input_sha256")

    targets = _require_closed_mapping(manifest["targets"], set(TARGETS), "targets")
    for target_name, expected in TARGETS.items():
        target = _require_closed_mapping(
            targets[target_name], TARGET_KEYS, f"targets.{target_name}"
        )
        for key, expected_value in expected.items():
            if target[key] != expected_value:
                raise ContractError(
                    f"targets.{target_name}.{key} must equal {expected_value!r}"
                )

        lock_path = _safe_relative_path(
            target["lock_file"], f"targets.{target_name}.lock_file"
        )
        receipt_path = _safe_relative_path(
            target["receipt_file"], f"targets.{target_name}.receipt_file"
        )
        if (
            lock_path.parts[0] != target_name
            or lock_path.name != "coding-adventures-my-pkg.opam.locked"
        ):
            raise ContractError(
                f"targets.{target_name}.lock_file must use its target directory"
            )
        if (
            receipt_path.parts[0] != target_name
            or receipt_path.name != "installed-packages.txt"
        ):
            raise ContractError(
                f"targets.{target_name}.receipt_file must use its target directory"
            )

        _require_digest(target["lock_sha256"], f"targets.{target_name}.lock_sha256")
        _require_digest(
            target["receipt_sha256"], f"targets.{target_name}.receipt_sha256"
        )
        runner_image = _require_closed_mapping(
            target["runner_image"],
            RUNNER_IMAGE_KEYS,
            f"targets.{target_name}.runner_image",
        )
        for key in RUNNER_IMAGE_KEYS:
            if not isinstance(runner_image[key], str) or not runner_image[key]:
                raise ContractError(
                    f"targets.{target_name}.runner_image.{key} must be nonempty"
                )


def _require_regular_file(path: Path, label: str) -> None:
    if path.is_symlink() or not path.is_file():
        raise ContractError(f"{label} must be a regular file: {path}")


def _verify_digest(path: Path, expected: str, label: str) -> None:
    actual = _sha256(path)
    if actual != expected:
        raise ContractError(
            f"{label} digest mismatch: expected {expected}, found {actual}"
        )


def _verify_lock_direct_versions(
    path: Path, versions: Mapping[str, str], target_name: str
) -> None:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        raise ContractError(f"cannot read {target_name} lock: {exc}") from exc
    for name, version in versions.items():
        if name == "opam":
            continue
        pattern = re.compile(
            rf'"{re.escape(name)}"\s*\{{[^}}]*=\s*"{re.escape(version)}"[^}}]*\}}'
        )
        if pattern.search(text) is None:
            raise ContractError(
                f"{target_name} lock omits exact direct dependency {name} {version}"
            )


def validate_workflow_text(manifest: Mapping[str, Any], workflow_text: str) -> None:
    """Validate security and identity-critical workflow text offline."""

    required_fragments = [
        "permissions:\n  contents: read",
        "fail-fast: false",
        f"actions/checkout@{manifest['actions']['checkout']}",
        f"ocaml/setup-ocaml@{manifest['actions']['setup_ocaml']}",
        f"#{manifest['opam_repository_commit']}",
        "opam-pin: false",
        "dune-cache: false",
        "windows-compiler: mingw",
        "--require-checksums",
        "ocaml-library",
        "ocaml-program",
        "BUILD_windows",
    ]
    for fragment in required_fragments:
        if fragment not in workflow_text:
            raise ContractError(f"workflow omits required fragment {fragment!r}")

    for target in manifest["targets"].values():
        if target["runner"] not in workflow_text:
            raise ContractError(f"workflow omits runner label {target['runner']!r}")
    for version in manifest["direct_versions"].values():
        if version not in workflow_text:
            raise ContractError(f"workflow omits exact version {version!r}")

    forbidden = {
        "continue-on-error:": "continue-on-error",
        "|| true": "conditional success",
        "secrets.": "repository secret use",
        "dune-cache: true": "project dependency cache",
        "opam-pin: true": "package pinning",
    }
    for text, label in forbidden.items():
        if text in workflow_text:
            raise ContractError(f"workflow contains forbidden {label}")

    for reference in USES_REFERENCE.findall(workflow_text):
        if "@" not in reference:
            raise ContractError(f"workflow action is not commit-pinned: {reference}")
        _, revision = reference.rsplit("@", 1)
        if HEX_40.fullmatch(revision) is None:
            raise ContractError(f"workflow action is not commit-pinned: {reference}")


def validate_repository(
    repo_root: Path, *, check_workflow: bool = True
) -> dict[str, Any]:
    """Validate all checked OCAML03 evidence without network access."""

    manifest = load_manifest(repo_root)
    fixture_root = repo_root / FIXTURE_ROOT_RELATIVE_PATH
    library_input = (
        repo_root
        / SCAFFOLD_ROOT_RELATIVE_PATH
        / "ocaml-library/coding-adventures-my-pkg.opam"
    )
    program_input = (
        repo_root
        / SCAFFOLD_ROOT_RELATIVE_PATH
        / "ocaml-program/coding-adventures-my-pkg.opam"
    )
    _require_regular_file(library_input, "library fixture input")
    _require_regular_file(program_input, "program fixture input")
    if library_input.read_bytes() != program_input.read_bytes():
        raise ContractError("library and program fixture opam inputs differ")
    _verify_digest(
        library_input,
        manifest["fixture_input_sha256"],
        "fixture input",
    )

    for target_name, target in manifest["targets"].items():
        lock_path = fixture_root / PurePosixPath(target["lock_file"])
        receipt_path = fixture_root / PurePosixPath(target["receipt_file"])
        _require_regular_file(lock_path, f"{target_name} lock")
        _require_regular_file(receipt_path, f"{target_name} receipt")
        _verify_digest(lock_path, target["lock_sha256"], f"{target_name} lock")
        _verify_digest(
            receipt_path,
            target["receipt_sha256"],
            f"{target_name} receipt",
        )
        _verify_lock_direct_versions(
            lock_path, manifest["direct_versions"], target_name
        )

    if check_workflow:
        workflow_path = repo_root / WORKFLOW_RELATIVE_PATH
        _require_regular_file(workflow_path, "OCaml workflow")
        try:
            workflow_text = workflow_path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as exc:
            raise ContractError(f"cannot read workflow {workflow_path}: {exc}") from exc
        validate_workflow_text(manifest, workflow_text)
    return manifest


def validate_tool_version_outputs(
    manifest: Mapping[str, Any], outputs: Mapping[str, str]
) -> None:
    """Require each runtime probe output to contain its exact reviewed version."""

    expected_names = set(manifest["direct_versions"])
    if set(outputs) != expected_names:
        raise ContractError(
            "tool output keys must equal "
            f"{sorted(expected_names)}, found {sorted(outputs)}"
        )
    for name, expected in manifest["direct_versions"].items():
        value = outputs[name].strip()
        if re.search(rf"(?<![\d.]){re.escape(expected)}(?![\d.])", value) is None:
            raise ContractError(
                f"{name} must report exact version {expected}, found {value!r}"
            )


RUNTIME_COMMANDS: Mapping[str, Sequence[str]] = {
    "opam": ("opam", "--version"),
    "ocaml": ("opam", "exec", "--", "ocamlc", "-version"),
    "dune": ("opam", "exec", "--", "dune", "--version"),
    "alcotest": (
        "opam",
        "list",
        "--installed",
        "--short",
        "--columns=version",
        "alcotest",
    ),
    "bisect_ppx": (
        "opam",
        "list",
        "--installed",
        "--short",
        "--columns=version",
        "bisect_ppx",
    ),
    "ocamlformat": (
        "opam",
        "list",
        "--installed",
        "--short",
        "--columns=version",
        "ocamlformat",
    ),
}


def validate_runtime(manifest: Mapping[str, Any]) -> None:
    """Run the closed read-only version probes for the current switch."""

    outputs: dict[str, str] = {}
    for name, command in RUNTIME_COMMANDS.items():
        completed = subprocess.run(
            list(command),
            check=True,
            shell=False,
            text=True,
            capture_output=True,
        )
        outputs[name] = completed.stdout
    validate_tool_version_outputs(manifest, outputs)


def _repo_root_from_script() -> Path:
    return Path(__file__).resolve().parents[2]


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate_parser = subparsers.add_parser(
        "validate-repository", help="validate checked evidence offline"
    )
    validate_parser.add_argument(
        "--repo-root", type=Path, default=_repo_root_from_script()
    )
    runtime_parser = subparsers.add_parser(
        "validate-runtime", help="validate exact tools in the current switch"
    )
    runtime_parser.add_argument(
        "--repo-root", type=Path, default=_repo_root_from_script()
    )
    args = parser.parse_args(argv)

    try:
        if args.command == "validate-repository":
            validate_repository(args.repo_root.resolve())
        else:
            manifest = load_manifest(args.repo_root.resolve())
            validate_runtime(manifest)
    except (ContractError, OSError, subprocess.CalledProcessError) as exc:
        print(f"OCAML03 validation failed: {exc}", file=sys.stderr)
        return 1
    print(f"OCAML03 {args.command} passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
