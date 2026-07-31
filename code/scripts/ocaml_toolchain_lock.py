#!/usr/bin/env python3
"""Validate the checked OCaml CI toolchain and transitive solver evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import stat
import subprocess  # nosec B404
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
    "upload_artifact": "ea165f8d65b6e75b540449e92b4886f43607fa02",
}
OPAM_REPOSITORY_COMMIT = "ba8cc66eb9e5baae7ebc88cf77f4c488d63d87ff"
TARGETS = {
    "linux-x64": {
        "runner": "ubuntu-24.04",
        "runner_os": "Linux",
        "runner_arch": "X64",
        "windows_compiler": None,
    },
    "macos-arm64": {
        "runner": "macos-14",
        "runner_os": "macOS",
        "runner_arch": "ARM64",
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
YAML_MAPPING_ENTRY = re.compile(r"^([A-Za-z0-9_-]+):(.*)$")
YAML_BLOCK_SENTINEL = "\0BLOCK\0"

EXPECTED_BUILD_LINES = {
    "BUILD": (
        "opam install . --deps-only --with-test --with-dev-setup -y",
        "opam exec -- dune build @fmt",
        (
            'BISECT_FILE="$PWD/bisect" opam exec -- dune runtest --force '
            "--instrument-with bisect_ppx"
        ),
        (
            "opam exec -- bisect-ppx-report summary --per-file "
            "--expect src/coding_adventures_my_pkg.ml bisect*.coverage"
        ),
    ),
    "BUILD_windows": (
        "opam install . --deps-only --with-test --with-dev-setup -y",
        "opam exec -- dune build @fmt",
        (
            "set BISECT_FILE=%CD%\\bisect&& opam exec -- dune runtest --force "
            "--instrument-with bisect_ppx"
        ),
        (
            "for %f in (bisect*.coverage) do opam exec -- "
            "bisect-ppx-report summary --per-file "
            "--expect src/coding_adventures_my_pkg.ml %f"
        ),
    ),
}
EXPECTED_RUN_SHA256 = {
    "contract.validate": "6e77be232d50974d1de5d950993e830f46792e4241804fbedf43fa75875c8ec7",
    "fresh.bootstrap": "240997bd7eff9f98a529c7410b2f8849a9f1dcfccc4edfafb8f8801db054f9ed",
    "fresh.generate": "1db0d48f6e40cf98a1b93dc9acaa5db9b84035be22c494d5483423b35ca45d26",
    "fresh.compare": "c5380cbbe8df09fc05e09d15f0c275e64d30f7d782e50a3fd771fab8b59cfb6b",
    "locked.bootstrap": "5c6306a2240b83375adf154cd035a4ad73f48288b4ee93ea97aaae6948f71eed",
    "locked.install": "83aaefbc943f71a9be36cd9cfa8ac70919040de0d0e71748f93999b84023172c",
    "locked.run": "68cdbecd8ce825ae00f1f1516f3ab73bd06198ac9e3f1c238f06518a00353929",
}
EXPECTED_RUN_METADATA = {
    "contract.validate": {"shell": None, "env": None},
    "fresh.bootstrap": {
        "shell": "bash",
        "env": {
            "EXPECTED_OS": "${{ matrix.runner-os }}",
            "EXPECTED_ARCH": "${{ matrix.runner-arch }}",
        },
    },
    "fresh.generate": {
        "shell": "bash",
        "env": {"TARGET": "${{ matrix.target }}"},
    },
    "fresh.compare": {
        "shell": "bash",
        "env": {"TARGET": "${{ matrix.target }}"},
    },
    "locked.bootstrap": {
        "shell": "bash",
        "env": {
            "EXPECTED_OS": "${{ matrix.runner-os }}",
            "EXPECTED_ARCH": "${{ matrix.runner-arch }}",
        },
    },
    "locked.install": {
        "shell": "bash",
        "env": {"TARGET": "${{ matrix.target }}"},
    },
    "locked.run": {
        "shell": "bash",
        "env": {"TARGET": "${{ matrix.target }}"},
    },
}


class ContractError(ValueError):
    """Raised when toolchain evidence violates OCAML03."""


def _yaml_scalar(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
        return value[1:-1]
    return value


def _tokenize_restricted_yaml(text: str) -> list[tuple[int, str, str | None, int]]:
    """Tokenize the workflow subset while keeping block scalars opaque."""

    physical_lines = text.splitlines()
    tokens: list[tuple[int, str, str | None, int]] = []
    index = 0
    while index < len(physical_lines):
        raw = physical_lines[index]
        line_number = index + 1
        if "\t" in raw:
            raise ContractError(f"workflow line {line_number} contains a tab")
        content = raw.lstrip(" ")
        if not content or content.startswith("#"):
            index += 1
            continue
        indent = len(raw) - len(content)
        if indent % 2:
            raise ContractError(
                f"workflow line {line_number} uses unsupported indentation"
            )

        mapping_text = content.removeprefix("- ")
        match = YAML_MAPPING_ENTRY.fullmatch(mapping_text)
        if match is not None and match.group(2).strip() == ">":
            raise ContractError(
                f"workflow line {line_number} uses unsupported folded block scalar"
            )
        if match is not None and match.group(2).strip() == "|":
            block_lines: list[str] = []
            index += 1
            while index < len(physical_lines):
                candidate = physical_lines[index]
                if not candidate.strip():
                    block_lines.append("")
                    index += 1
                    continue
                candidate_indent = len(candidate) - len(candidate.lstrip(" "))
                if candidate_indent <= indent:
                    break
                block_lines.append(candidate)
                index += 1
            nonblank_indents = [
                len(line) - len(line.lstrip(" "))
                for line in block_lines
                if line.strip()
            ]
            block_indent = min(nonblank_indents, default=indent + 2)
            block_value = "\n".join(
                line[block_indent:] if line else "" for line in block_lines
            ).rstrip()
            prefix = "- " if content.startswith("- ") else ""
            tokens.append(
                (
                    indent,
                    f"{prefix}{match.group(1)}: {YAML_BLOCK_SENTINEL}",
                    block_value,
                    line_number,
                )
            )
            continue

        tokens.append((indent, content, None, line_number))
        index += 1
    return tokens


def _split_yaml_mapping(content: str, line_number: int) -> tuple[str, str]:
    match = YAML_MAPPING_ENTRY.fullmatch(content)
    if match is None:
        raise ContractError(
            f"workflow line {line_number} is not a supported mapping entry"
        )
    return match.group(1), match.group(2).strip()


def _parse_restricted_yaml_block(
    tokens: Sequence[tuple[int, str, str | None, int]],
    position: int,
    indent: int,
) -> tuple[object, int]:
    if position >= len(tokens) or tokens[position][0] != indent:
        raise ContractError("workflow has malformed indentation")
    is_sequence = tokens[position][1].startswith("- ")

    if is_sequence:
        result: list[object] = []
        while position < len(tokens):
            token_indent, content, block_value, line_number = tokens[position]
            if token_indent < indent:
                break
            if token_indent != indent or not content.startswith("- "):
                raise ContractError(
                    f"workflow line {line_number} has malformed sequence indentation"
                )
            remainder = content[2:].strip()
            position += 1
            if not remainder:
                if position >= len(tokens) or tokens[position][0] <= indent:
                    raise ContractError(
                        f"workflow line {line_number} has an empty sequence item"
                    )
                item, position = _parse_restricted_yaml_block(
                    tokens, position, tokens[position][0]
                )
                result.append(item)
                continue

            if YAML_MAPPING_ENTRY.fullmatch(remainder) is None:
                result.append(_yaml_scalar(remainder))
                continue

            key, scalar = _split_yaml_mapping(remainder, line_number)
            item_mapping: dict[str, object] = {}
            if scalar == YAML_BLOCK_SENTINEL:
                item_mapping[key] = block_value or ""
            elif scalar:
                item_mapping[key] = _yaml_scalar(scalar)
            elif position < len(tokens) and tokens[position][0] > indent:
                item_mapping[key], position = _parse_restricted_yaml_block(
                    tokens, position, tokens[position][0]
                )
            else:
                item_mapping[key] = None

            if position < len(tokens) and tokens[position][0] > indent:
                continuation_indent = tokens[position][0]
                continuation, position = _parse_restricted_yaml_block(
                    tokens, position, continuation_indent
                )
                if not isinstance(continuation, dict):
                    raise ContractError(
                        f"workflow line {line_number} has a non-mapping continuation"
                    )
                overlap = set(item_mapping) & set(continuation)
                if overlap:
                    raise ContractError(
                        f"workflow line {line_number} duplicates keys {sorted(overlap)}"
                    )
                item_mapping.update(continuation)
            result.append(item_mapping)
        return result, position

    result_mapping: dict[str, object] = {}
    while position < len(tokens):
        token_indent, content, block_value, line_number = tokens[position]
        if token_indent < indent:
            break
        if token_indent != indent or content.startswith("- "):
            raise ContractError(
                f"workflow line {line_number} has malformed mapping indentation"
            )
        key, scalar = _split_yaml_mapping(content, line_number)
        if key in result_mapping:
            raise ContractError(f"workflow line {line_number} duplicates {key!r}")
        position += 1
        if scalar == YAML_BLOCK_SENTINEL:
            result_mapping[key] = block_value or ""
        elif scalar:
            result_mapping[key] = _yaml_scalar(scalar)
        elif position < len(tokens) and tokens[position][0] > indent:
            result_mapping[key], position = _parse_restricted_yaml_block(
                tokens, position, tokens[position][0]
            )
        else:
            result_mapping[key] = None
    return result_mapping, position


def parse_restricted_workflow_yaml(text: str) -> dict[str, object]:
    """Parse the deliberately small YAML subset used by build-ocaml.yml."""

    tokens = _tokenize_restricted_yaml(text)
    if not tokens:
        raise ContractError("workflow is empty")
    if tokens[0][0] != 0:
        raise ContractError("workflow root must start at indentation zero")
    document, position = _parse_restricted_yaml_block(tokens, 0, 0)
    if position != len(tokens) or not isinstance(document, dict):
        raise ContractError("workflow root must be one closed mapping")
    return document


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
    if (
        path.is_absolute()
        or path.as_posix() != value
        or any(part in {"", ".", ".."} for part in path.parts)
    ):
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
    if not path.exists():
        raise ContractError(f"missing manifest: {path}")
    _require_regular_file(path, "manifest", boundary=repo_root)
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
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


def _is_link_or_reparse_point(path: Path) -> bool:
    try:
        details = path.lstat()
    except OSError as exc:
        raise ContractError(f"cannot inspect path component {path}: {exc}") from exc
    attributes = getattr(details, "st_file_attributes", 0)
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return path.is_symlink() or bool(attributes & reparse_flag)


def _require_regular_file(path: Path, label: str, *, boundary: Path) -> None:
    if not path.exists() and not path.is_symlink():
        raise ContractError(f"{label} must be a regular file: {path}")
    try:
        relative = path.relative_to(boundary)
    except ValueError as exc:
        raise ContractError(f"{label} escapes its reviewed boundary: {path}") from exc

    current = boundary
    for part in relative.parts:
        current /= part
        if _is_link_or_reparse_point(current):
            raise ContractError(f"{label} contains a linked path component: {current}")

    try:
        boundary_resolved = boundary.resolve(strict=True)
        resolved = path.resolve(strict=True)
        resolved.relative_to(boundary_resolved)
    except (OSError, ValueError) as exc:
        raise ContractError(f"{label} escapes its reviewed boundary: {path}") from exc
    if not stat.S_ISREG(path.stat().st_mode):
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


def _validate_scaffold_build_files(repo_root: Path) -> None:
    for kind in ("ocaml-library", "ocaml-program"):
        for filename, expected_lines in EXPECTED_BUILD_LINES.items():
            path = repo_root / SCAFFOLD_ROOT_RELATIVE_PATH / kind / filename
            label = f"{kind} {filename}"
            _require_regular_file(path, label, boundary=repo_root)
            try:
                actual_lines = tuple(path.read_text(encoding="utf-8").splitlines())
            except (OSError, UnicodeError) as exc:
                raise ContractError(f"cannot read {label}: {exc}") from exc
            if actual_lines != expected_lines:
                raise ContractError(
                    f"{label} must equal the reviewed format, test, and coverage commands"
                )


def _workflow_mapping(value: object, label: str) -> Mapping[str, object]:
    if not isinstance(value, dict):
        raise ContractError(f"workflow {label} must be a mapping")
    return value


def _workflow_sequence(value: object, label: str) -> Sequence[object]:
    if not isinstance(value, list):
        raise ContractError(f"workflow {label} must be a sequence")
    return value


def _workflow_string(value: object, label: str) -> str:
    if not isinstance(value, str):
        raise ContractError(f"workflow {label} must be a string")
    return value


def _require_workflow_keys(
    value: Mapping[str, object], expected: set[str], label: str
) -> None:
    actual = set(value)
    if actual != expected:
        raise ContractError(
            f"workflow {label} keys must equal {sorted(expected)}, "
            f"found {sorted(actual)}"
        )


def _steps_by_name(job: Mapping[str, object], job_name: str) -> dict[str, object]:
    steps = _workflow_sequence(job.get("steps"), f"jobs.{job_name}.steps")
    result: dict[str, object] = {}
    for index, value in enumerate(steps):
        step = _workflow_mapping(value, f"jobs.{job_name}.steps[{index}]")
        name = _workflow_string(
            step.get("name"), f"jobs.{job_name}.steps[{index}].name"
        )
        if name in result:
            raise ContractError(f"workflow jobs.{job_name} duplicates step {name!r}")
        result[name] = step
    return result


def _validate_matrix(
    manifest: Mapping[str, Any], job: Mapping[str, object], job_name: str
) -> None:
    strategy = _workflow_mapping(job.get("strategy"), f"jobs.{job_name}.strategy")
    _require_workflow_keys(
        strategy, {"fail-fast", "matrix"}, f"jobs.{job_name}.strategy"
    )
    if strategy["fail-fast"] != "false":
        raise ContractError(f"workflow jobs.{job_name} must disable fail-fast")
    matrix = _workflow_mapping(strategy["matrix"], f"jobs.{job_name}.strategy.matrix")
    _require_workflow_keys(matrix, {"include"}, f"jobs.{job_name}.strategy.matrix")
    include = _workflow_sequence(
        matrix["include"], f"jobs.{job_name}.strategy.matrix.include"
    )
    expected = [
        {
            "target": target_name,
            "runner": target["runner"],
            "runner-os": target["runner_os"],
            "runner-arch": target["runner_arch"],
        }
        for target_name, target in manifest["targets"].items()
    ]
    if list(include) != expected:
        raise ContractError(
            f"workflow jobs.{job_name} matrix must equal the reviewed target matrix"
        )


def _validate_checkout_step(
    step: Mapping[str, object], manifest: Mapping[str, Any], label: str
) -> None:
    _require_workflow_keys(step, {"name", "uses", "with"}, label)
    expected = f"actions/checkout@{manifest['actions']['checkout']}"
    if step["uses"] != expected:
        raise ContractError(f"workflow {label} must use {expected}")
    inputs = _workflow_mapping(step["with"], f"{label}.with")
    if inputs != {"persist-credentials": "false"}:
        raise ContractError(
            f"workflow {label} must disable persisted checkout credentials"
        )


def _validate_setup_step(
    step: Mapping[str, object],
    manifest: Mapping[str, Any],
    label: str,
    phase: str,
) -> None:
    _require_workflow_keys(step, {"name", "uses", "with"}, label)
    expected_action = f"ocaml/setup-ocaml@{manifest['actions']['setup_ocaml']}"
    if step["uses"] != expected_action:
        raise ContractError(f"workflow {label} must use {expected_action}")
    inputs = _workflow_mapping(step["with"], f"{label}.with")
    expected_inputs = {
        "ocaml-compiler": (
            f"ocaml-base-compiler.{manifest['direct_versions']['ocaml']}"
        ),
        "opam-repositories": (
            "default: git+https://github.com/ocaml/opam-repository.git"
            f"#{manifest['opam_repository_commit']}"
        ),
        "opam-pin": "false",
        "dune-cache": "false",
        "cache-prefix": (
            f"ocaml03-${{{{ github.sha }}}}-${{{{ github.run_id }}}}-"
            f"${{{{ github.run_attempt }}}}-{phase}-${{{{ matrix.target }}}}"
        ),
        "windows-compiler": "mingw",
        "windows-environment": "cygwin",
        "github-token": "${{ github.token }}",
    }
    if inputs != expected_inputs:
        raise ContractError(
            f"workflow {label} inputs must equal the reviewed setup inputs"
        )


def _validate_upload_step(
    step: Mapping[str, object],
    manifest: Mapping[str, Any],
    label: str,
    artifact_kind: str,
) -> None:
    _require_workflow_keys(step, {"name", "uses", "with"}, label)
    expected = f"actions/upload-artifact@{manifest['actions']['upload_artifact']}"
    if step["uses"] != expected:
        raise ContractError(f"workflow {label} must use {expected}")
    inputs = _workflow_mapping(step["with"], f"{label}.with")
    expected_inputs = {
        "name": f"ocaml03-{artifact_kind}-${{{{ matrix.target }}}}",
        "path": (
            f"${{{{ runner.temp }}}}/ocaml03-"
            f"{'evidence' if artifact_kind == 'fresh' else 'coverage'}/"
            "${{ matrix.target }}"
        ),
        "if-no-files-found": "error",
        "retention-days": "7",
    }
    if inputs != expected_inputs:
        raise ContractError(
            f"workflow {label} inputs must equal the reviewed artifact inputs"
        )


def _require_run_fragments(
    step: Mapping[str, object],
    label: str,
    fragments: Sequence[str],
    expected_digest_name: str,
) -> None:
    allowed = {"name", "run", "shell", "env"}
    if not set(step).issubset(allowed) or not {"name", "run"}.issubset(step):
        raise ContractError(f"workflow {label} has unreviewed run-step keys")
    run = _workflow_string(step["run"], f"{label}.run")
    metadata = EXPECTED_RUN_METADATA[expected_digest_name]
    expected_keys = {"name", "run"}
    if metadata["shell"] is not None:
        expected_keys.add("shell")
    if metadata["env"] is not None:
        expected_keys.add("env")
    _require_workflow_keys(step, expected_keys, label)
    if metadata["shell"] is not None and step["shell"] != metadata["shell"]:
        raise ContractError(f"workflow {label} shell must equal {metadata['shell']!r}")
    if metadata["env"] is not None:
        env = _workflow_mapping(step["env"], f"{label}.env")
        if env != metadata["env"]:
            raise ContractError(f"workflow {label} environment drifted")
    for fragment in fragments:
        if fragment not in run:
            raise ContractError(
                f"workflow {label} omits required fragment {fragment!r}"
            )
    actual_digest = hashlib.sha256(run.encode("utf-8")).hexdigest()
    expected_digest = EXPECTED_RUN_SHA256[expected_digest_name]
    if actual_digest != expected_digest:
        raise ContractError(
            f"workflow {label} run block must equal reviewed digest "
            f"{expected_digest}, found {actual_digest}"
        )


def _collect_uses(value: object) -> list[str]:
    references: list[str] = []
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "uses":
                references.append(_workflow_string(child, "uses"))
            references.extend(_collect_uses(child))
    elif isinstance(value, list):
        for child in value:
            references.extend(_collect_uses(child))
    return references


def validate_workflow_text(manifest: Mapping[str, Any], workflow_text: str) -> None:
    """Structurally validate the closed OCAML03 workflow offline."""

    document = parse_restricted_workflow_yaml(workflow_text)
    expected_uses = sorted(
        [f"actions/checkout@{manifest['actions']['checkout']}"] * 3
        + [f"ocaml/setup-ocaml@{manifest['actions']['setup_ocaml']}"] * 2
        + [f"actions/upload-artifact@{manifest['actions']['upload_artifact']}"] * 2
    )
    if sorted(_collect_uses(document)) != expected_uses:
        raise ContractError("workflow actions must equal the reviewed action allowlist")

    _require_workflow_keys(
        document, {"name", "on", "permissions", "env", "jobs"}, "root"
    )
    if document["name"] != "OCaml toolchain":
        raise ContractError("workflow name must equal the reviewed identity")
    triggers = _workflow_mapping(document["on"], "on")
    _require_workflow_keys(
        triggers, {"push", "pull_request", "workflow_dispatch"}, "on"
    )
    expected_paths = [
        ".github/workflows/build-ocaml.yml",
        "code/scripts/ocaml_toolchain_lock.py",
        "code/scripts/tests/test_ocaml_toolchain_lock.py",
        "code/specs/OCAML0*.md",
        "code/specs/fixtures/ocaml-toolchain/**",
        "code/specs/fixtures/scaffold-generator/ocaml-*/**",
        "code/packages/ocaml/**",
        "code/programs/ocaml/**",
    ]
    for trigger_name, expected_branches in (
        ("push", "['**']"),
        ("pull_request", "[main]"),
    ):
        trigger = _workflow_mapping(triggers[trigger_name], f"on.{trigger_name}")
        _require_workflow_keys(trigger, {"branches", "paths"}, f"on.{trigger_name}")
        if (
            trigger["branches"] != expected_branches
            or trigger["paths"] != expected_paths
        ):
            raise ContractError(
                f"workflow on.{trigger_name} must equal the reviewed trigger"
            )
    if triggers["workflow_dispatch"] is not None:
        raise ContractError("workflow_dispatch must not accept unreviewed inputs")

    permissions = _workflow_mapping(document.get("permissions"), "permissions")
    if permissions != {"contents": "read"}:
        raise ContractError("workflow permissions must equal read-only contents")

    env = _workflow_mapping(document.get("env"), "env")
    expected_env = {
        "OCAML_VERSION": manifest["direct_versions"]["ocaml"],
        "OPAM_VERSION": manifest["direct_versions"]["opam"],
        "DUNE_VERSION": manifest["direct_versions"]["dune"],
        "ALCOTEST_VERSION": manifest["direct_versions"]["alcotest"],
        "BISECT_PPX_VERSION": manifest["direct_versions"]["bisect_ppx"],
        "OCAMLFORMAT_VERSION": manifest["direct_versions"]["ocamlformat"],
        "OPAM_REPOSITORY_COMMIT": manifest["opam_repository_commit"],
    }
    if env != expected_env:
        raise ContractError("workflow environment must equal reviewed identities")

    jobs = _workflow_mapping(document.get("jobs"), "jobs")
    _require_workflow_keys(jobs, {"contract", "fresh-solve", "locked-fixtures"}, "jobs")
    contract = _workflow_mapping(jobs["contract"], "jobs.contract")
    fresh = _workflow_mapping(jobs["fresh-solve"], "jobs.fresh-solve")
    locked = _workflow_mapping(jobs["locked-fixtures"], "jobs.locked-fixtures")
    _require_workflow_keys(contract, {"name", "runs-on", "steps"}, "jobs.contract")
    _require_workflow_keys(
        fresh, {"name", "needs", "runs-on", "strategy", "steps"}, "jobs.fresh-solve"
    )
    _require_workflow_keys(
        locked,
        {"name", "needs", "runs-on", "strategy", "steps"},
        "jobs.locked-fixtures",
    )
    if contract["runs-on"] != "ubuntu-24.04":
        raise ContractError("workflow contract job must use ubuntu-24.04")
    if (
        fresh["needs"] != "contract"
        or fresh["runs-on"] != "${{ matrix.runner }}"
        or locked["needs"] != "fresh-solve"
        or locked["runs-on"] != "${{ matrix.runner }}"
    ):
        raise ContractError("workflow job dependency chain or runner binding drifted")
    _validate_matrix(manifest, fresh, "fresh-solve")
    _validate_matrix(manifest, locked, "locked-fixtures")

    contract_steps = _steps_by_name(contract, "contract")
    fresh_steps = _steps_by_name(fresh, "fresh-solve")
    locked_steps = _steps_by_name(locked, "locked-fixtures")
    if list(contract_steps) != [
        "Checkout reviewed source",
        "Validate closed manifest, evidence, and workflow",
    ]:
        raise ContractError("workflow contract steps must equal the reviewed sequence")
    if list(fresh_steps) != [
        "Checkout reviewed source",
        "Set up the reviewed OCaml compiler and repository",
        "Require the reviewed runner and opam bootstrap",
        "Generate fresh solver evidence",
        "Upload fresh solver evidence",
        "Compare fresh solve with reviewed evidence",
    ]:
        raise ContractError(
            "workflow fresh-solve steps must equal the reviewed sequence"
        )
    if list(locked_steps) != [
        "Checkout reviewed source",
        "Set up the reviewed OCaml compiler and repository",
        "Require the reviewed runner and opam bootstrap",
        "Install the reviewed transitive lock",
        "Run both scaffold kinds line by line",
        "Upload measured coverage",
    ]:
        raise ContractError(
            "workflow locked-fixtures steps must equal the reviewed sequence"
        )

    for job_name, steps in (
        ("contract", contract_steps),
        ("fresh-solve", fresh_steps),
        ("locked-fixtures", locked_steps),
    ):
        _validate_checkout_step(
            _workflow_mapping(
                steps["Checkout reviewed source"],
                f"jobs.{job_name}.checkout",
            ),
            manifest,
            f"jobs.{job_name}.checkout",
        )
    _validate_setup_step(
        _workflow_mapping(
            fresh_steps["Set up the reviewed OCaml compiler and repository"],
            "jobs.fresh-solve.setup",
        ),
        manifest,
        "jobs.fresh-solve.setup",
        "fresh",
    )
    _validate_setup_step(
        _workflow_mapping(
            locked_steps["Set up the reviewed OCaml compiler and repository"],
            "jobs.locked-fixtures.setup",
        ),
        manifest,
        "jobs.locked-fixtures.setup",
        "locked",
    )
    _validate_upload_step(
        _workflow_mapping(
            fresh_steps["Upload fresh solver evidence"],
            "jobs.fresh-solve.upload",
        ),
        manifest,
        "jobs.fresh-solve.upload",
        "fresh",
    )
    _validate_upload_step(
        _workflow_mapping(
            locked_steps["Upload measured coverage"],
            "jobs.locked-fixtures.upload",
        ),
        manifest,
        "jobs.locked-fixtures.upload",
        "coverage",
    )

    _require_run_fragments(
        _workflow_mapping(
            contract_steps["Validate closed manifest, evidence, and workflow"],
            "jobs.contract.validate",
        ),
        "jobs.contract.validate",
        ("validate-repository", "test_ocaml_toolchain_lock.py"),
        "contract.validate",
    )
    for job_name, steps in (
        ("fresh-solve", fresh_steps),
        ("locked-fixtures", locked_steps),
    ):
        _require_run_fragments(
            _workflow_mapping(
                steps["Require the reviewed runner and opam bootstrap"],
                f"jobs.{job_name}.bootstrap",
            ),
            f"jobs.{job_name}.bootstrap",
            (
                'test "$RUNNER_OS" = "$EXPECTED_OS"',
                'test "$RUNNER_ARCH" = "$EXPECTED_ARCH"',
                'test "$actual" = "$OPAM_VERSION"',
                "opam repository list --all --short --color=never",
                "opam repository list --all --color=never",
                'test "$repository_names" = "default"',
                "validate-repository-report",
                '--names "$repository_names"',
                '--listing "$repository_listing"',
                "dune alcotest bisect_ppx ocamlformat",
            ),
            f"{'fresh' if job_name == 'fresh-solve' else 'locked'}.bootstrap",
        )
    _require_run_fragments(
        _workflow_mapping(
            fresh_steps["Generate fresh solver evidence"],
            "jobs.fresh-solve.generate",
        ),
        "jobs.fresh-solve.generate",
        (
            "--require-checksums",
            "opam lock .",
            "installed-packages.txt",
            "validate-runtime",
        ),
        "fresh.generate",
    )
    _require_run_fragments(
        _workflow_mapping(
            fresh_steps["Compare fresh solve with reviewed evidence"],
            "jobs.fresh-solve.compare",
        ),
        "jobs.fresh-solve.compare",
        ("diff -u", "coding-adventures-my-pkg.opam.locked", "installed-packages.txt"),
        "fresh.compare",
    )
    _require_run_fragments(
        _workflow_mapping(
            locked_steps["Install the reviewed transitive lock"],
            "jobs.locked-fixtures.install",
        ),
        "jobs.locked-fixtures.install",
        ("--locked", "--require-checksums", "validate-runtime"),
        "locked.install",
    )
    _require_run_fragments(
        _workflow_mapping(
            locked_steps["Run both scaffold kinds line by line"],
            "jobs.locked-fixtures.run",
        ),
        "jobs.locked-fixtures.run",
        (
            "export OPAMLOCKED=true",
            "export OPAMREQUIRECHECKSUMS=true",
            "coding-adventures-my-pkg.opam.locked",
            "ocaml-library ocaml-program",
            "BUILD_windows",
            'read -r command || [ -n "$command" ]',
            'cmd.exe /D /S /C "$command"',
            'sh -c "$command"',
            "bisect*.coverage",
            'diff -u "$expected_receipt"',
        ),
        "locked.run",
    )

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
    if workflow_text.count("${{ github.token }}") != 2:
        raise ContractError(
            "workflow automatic GitHub token use must be limited to the two "
            "reviewed setup inputs"
        )


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
    _require_regular_file(library_input, "library fixture input", boundary=repo_root)
    _require_regular_file(program_input, "program fixture input", boundary=repo_root)
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
        _require_regular_file(lock_path, f"{target_name} lock", boundary=repo_root)
        _require_regular_file(
            receipt_path, f"{target_name} receipt", boundary=repo_root
        )
        _verify_digest(lock_path, target["lock_sha256"], f"{target_name} lock")
        _verify_digest(
            receipt_path,
            target["receipt_sha256"],
            f"{target_name} receipt",
        )
        _verify_lock_direct_versions(
            lock_path, manifest["direct_versions"], target_name
        )

    _validate_scaffold_build_files(repo_root)

    if check_workflow:
        workflow_path = repo_root / WORKFLOW_RELATIVE_PATH
        _require_regular_file(workflow_path, "OCaml workflow", boundary=repo_root)
        try:
            workflow_text = workflow_path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as exc:
            raise ContractError(f"cannot read workflow {workflow_path}: {exc}") from exc
        validate_workflow_text(manifest, workflow_text)
    return manifest


def validate_tool_version_outputs(
    manifest: Mapping[str, Any], outputs: Mapping[str, str]
) -> None:
    """Require each runtime probe output to equal its exact reviewed version."""

    expected_names = set(manifest["direct_versions"])
    if set(outputs) != expected_names:
        raise ContractError(
            "tool output keys must equal "
            f"{sorted(expected_names)}, found {sorted(outputs)}"
        )
    for name, expected in manifest["direct_versions"].items():
        value = outputs[name].strip()
        if value != expected:
            raise ContractError(
                f"{name} must report exact version {expected}, found {value!r}"
            )


def validate_repository_report(
    manifest: Mapping[str, Any], names: str, listing: str
) -> None:
    """Validate opam's color-disabled configured-repository report."""

    if names.splitlines() != ["default"]:
        raise ContractError("opam must report exactly the default repository")
    expected_source = (
        "git+https://github.com/ocaml/opam-repository.git"
        f"#{manifest['opam_repository_commit']}"
    )
    default_rows = [
        fields
        for line in listing.splitlines()
        if (fields := line.split()) and fields[0] == "default"
    ]
    if (
        len(default_rows) != 1
        or len(default_rows[0]) < 2
        or default_rows[0][1] != expected_source
        or listing.count(expected_source) != 1
    ):
        raise ContractError(
            "opam default repository must equal the reviewed commit-qualified source"
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
        "--color=never",
        "--columns=version",
        "alcotest",
    ),
    "bisect_ppx": (
        "opam",
        "list",
        "--installed",
        "--short",
        "--color=never",
        "--columns=version",
        "bisect_ppx",
    ),
    "ocamlformat": (
        "opam",
        "list",
        "--installed",
        "--short",
        "--color=never",
        "--columns=version",
        "ocamlformat",
    ),
}


def validate_runtime(manifest: Mapping[str, Any]) -> None:
    """Run the closed read-only version probes for the current switch."""

    outputs: dict[str, str] = {}
    for name, command in RUNTIME_COMMANDS.items():
        # Every command is a fixed tuple from RUNTIME_COMMANDS.
        completed = subprocess.run(  # nosec B603
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
    report_parser = subparsers.add_parser(
        "validate-repository-report",
        help="validate opam's configured-repository report",
    )
    report_parser.add_argument(
        "--repo-root", type=Path, default=_repo_root_from_script()
    )
    report_parser.add_argument("--names", required=True)
    report_parser.add_argument("--listing", required=True)
    args = parser.parse_args(argv)

    try:
        if args.command == "validate-repository":
            validate_repository(args.repo_root.resolve())
        elif args.command == "validate-runtime":
            manifest = load_manifest(args.repo_root.resolve())
            validate_runtime(manifest)
        else:
            manifest = load_manifest(args.repo_root.resolve())
            validate_repository_report(manifest, args.names, args.listing)
    except (ContractError, OSError, subprocess.CalledProcessError) as exc:
        print(f"OCAML03 validation failed: {exc}", file=sys.stderr)
        return 1
    print(f"OCAML03 {args.command} passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
