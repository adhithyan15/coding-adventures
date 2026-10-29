#!/usr/bin/env python3
"""Enforce the repository's monotone third-party dependency inventory."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tomllib
from collections.abc import Iterable, Mapping
from pathlib import Path
from typing import Any


DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
DEFAULT_ALLOWLIST = Path("code/specs/data/third-party-dependencies/rust-allowlist.json")


class PolicyError(ValueError):
    """Raised when an inventory or allowlist violates the policy contract."""


def git_cargo_manifests(root: Path) -> list[Path]:
    """Return every tracked Cargo manifest, excluding generated files."""

    result = subprocess.run(
        ["git", "ls-files", "-z", "--", ":(glob)**/Cargo.toml"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    return sorted(
        root / raw_path.decode("utf-8", errors="surrogateescape")
        for raw_path in result.stdout.split(b"\0")
        if raw_path
    )


def dependency_tables(document: Mapping[str, Any]) -> Iterable[Mapping[str, Any]]:
    """Yield Cargo dependency tables, including workspace and target scopes."""

    for table_name in DEPENDENCY_TABLES:
        table = document.get(table_name)
        if isinstance(table, Mapping):
            yield table

    workspace = document.get("workspace")
    if isinstance(workspace, Mapping):
        table = workspace.get("dependencies")
        if isinstance(table, Mapping):
            yield table

    targets = document.get("target")
    if isinstance(targets, Mapping):
        for target in targets.values():
            if not isinstance(target, Mapping):
                continue
            for table_name in DEPENDENCY_TABLES:
                table = target.get(table_name)
                if isinstance(table, Mapping):
                    yield table


def third_party_name(
    alias: str,
    declaration: Any,
    *,
    manifest_path: Path | None = None,
    repository_root: Path | None = None,
    tracked_manifests: set[Path] | None = None,
) -> str | None:
    """Return the registry/git package name, or None for a local/workspace edge."""

    if isinstance(declaration, str):
        return alias
    if not isinstance(declaration, Mapping):
        raise PolicyError(f"dependency {alias!r} has an unsupported declaration")
    dependency_path = declaration.get("path")
    if dependency_path is not None:
        if not isinstance(dependency_path, str) or not dependency_path:
            raise PolicyError(f"dependency {alias!r} has an invalid path")
        if (
            manifest_path is None
            or repository_root is None
            or tracked_manifests is None
        ):
            raise PolicyError(f"dependency {alias!r} path cannot be verified")
        candidate = (manifest_path.parent / dependency_path / "Cargo.toml").resolve()
        try:
            candidate.relative_to(repository_root.resolve())
        except ValueError:
            pass
        else:
            if candidate in tracked_manifests:
                return None
    if declaration.get("workspace") is True and not any(
        key in declaration for key in ("version", "git", "registry", "package")
    ):
        # The governing declaration is counted in [workspace.dependencies].
        return None
    package = declaration.get("package", alias)
    if not isinstance(package, str) or not package:
        raise PolicyError(f"dependency {alias!r} has an invalid package name")
    return package


def manifest_dependency_names(
    path: Path,
    repository_root: Path | None = None,
    tracked_manifests: set[Path] | None = None,
) -> set[str]:
    """Parse one Cargo manifest and return its third-party package names."""

    try:
        if path.is_file():
            with path.open("rb") as manifest:
                document = tomllib.load(manifest)
        elif repository_root is not None:
            relative_path = path.relative_to(repository_root).as_posix()
            result = subprocess.run(
                ["git", "show", f":{relative_path}"],
                cwd=repository_root,
                check=True,
                capture_output=True,
            )
            document = tomllib.loads(result.stdout.decode("utf-8"))
        else:
            raise FileNotFoundError(path)
    except (
        OSError,
        UnicodeDecodeError,
        subprocess.CalledProcessError,
        tomllib.TOMLDecodeError,
    ) as error:
        raise PolicyError(f"cannot parse {path}: {error}") from error

    names: set[str] = set()
    for table in dependency_tables(document):
        for alias, declaration in table.items():
            name = third_party_name(
                alias,
                declaration,
                manifest_path=path,
                repository_root=repository_root,
                tracked_manifests=tracked_manifests,
            )
            if name is not None:
                names.add(name)
    return names


def build_inventory(root: Path, manifests: Iterable[Path]) -> dict[str, list[str]]:
    """Map each third-party package name to the manifests declaring it."""

    inventory: dict[str, list[str]] = {}
    manifest_list = list(manifests)
    tracked_manifests = {manifest.resolve() for manifest in manifest_list}
    for manifest in manifest_list:
        try:
            display_path = manifest.relative_to(root).as_posix()
        except ValueError:
            display_path = manifest.as_posix()
        for name in manifest_dependency_names(manifest, root, tracked_manifests):
            inventory.setdefault(name, []).append(display_path)
    return {name: sorted(paths) for name, paths in sorted(inventory.items())}


def validate_allowlist(document: Any, label: str) -> list[str]:
    """Structurally validate an allowlist JSON document."""

    if not isinstance(document, Mapping):
        raise PolicyError(f"{label} must contain a JSON object")
    if document.get("schema_version") != 1 or document.get("ecosystem") != "cargo":
        raise PolicyError(f"{label} must be a schema_version 1 cargo allowlist")
    names = document.get("allowed_dependency_names")
    if not isinstance(names, list) or not all(
        isinstance(name, str) and name for name in names
    ):
        raise PolicyError(f"{label} allowed_dependency_names must be strings")
    if names != sorted(set(names)):
        raise PolicyError(f"{label} allowed_dependency_names must be sorted and unique")
    return names


def load_allowlist(path: Path) -> list[str]:
    """Load and structurally validate the checked dependency-name allowlist."""

    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise PolicyError(f"cannot read {path}: {error}") from error
    return validate_allowlist(document, str(path))


def load_base_allowlist(root: Path, ref: str, path: Path) -> list[str] | None:
    """Load the allowlist at a trusted Git ref, if that ref predates the gate."""

    revision = subprocess.run(
        ["git", "rev-parse", "--verify", ref],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
    )
    if revision.returncode != 0:
        raise PolicyError(f"cannot resolve dependency-policy base ref {ref!r}")
    listing = subprocess.run(
        ["git", "ls-tree", "--name-only", ref, "--", path.as_posix()],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    if not listing.stdout.strip():
        return None
    result = subprocess.run(
        ["git", "show", f"{ref}:{path.as_posix()}"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    try:
        document = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise PolicyError(f"cannot read {ref}:{path}: {error}") from error
    return validate_allowlist(document, f"{ref}:{path}")


def policy_differences(
    inventory: Mapping[str, list[str]], allowed_names: Iterable[str]
) -> tuple[list[str], list[str]]:
    """Return newly introduced and stale allowlist names."""

    present = set(inventory)
    allowed = set(allowed_names)
    return sorted(present - allowed), sorted(allowed - present)


def render_failure(
    inventory: Mapping[str, list[str]],
    unknown: list[str],
    stale: list[str],
    allowlist_growth: list[str],
) -> str:
    lines = ["Cargo third-party dependency policy failed."]
    if unknown:
        lines.append("New dependency names require explicit review:")
        for name in unknown:
            lines.append(f"  {name}: {', '.join(inventory[name])}")
    if stale:
        lines.append("Removed dependency names must be deleted from the allowlist:")
        lines.extend(f"  {name}" for name in stale)
    if allowlist_growth:
        lines.append("The approved dependency set may only shrink:")
        lines.extend(f"  {name}" for name in allowlist_growth)
    return "\n".join(lines)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("check", "inventory"))
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument("--allowlist", type=Path)
    parser.add_argument(
        "--base-ref",
        default=os.environ.get("THIRD_PARTY_DEPENDENCY_BASE_REF"),
        help="trusted Git ref whose allowlist is the monotone upper bound",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    root = args.root.resolve()
    try:
        manifests = git_cargo_manifests(root)
        inventory = build_inventory(root, manifests)
    except (PolicyError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        return 1
    if args.command == "inventory":
        print(json.dumps({"dependency_names": sorted(inventory)}, indent=2))
        return 0

    allowlist_path = args.allowlist or root / DEFAULT_ALLOWLIST
    try:
        allowed_names = load_allowlist(allowlist_path)
        unknown, stale = policy_differences(inventory, allowed_names)
        baseline = (
            load_base_allowlist(root, args.base_ref, DEFAULT_ALLOWLIST)
            if args.base_ref
            else None
        )
    except PolicyError as error:
        print(error, file=sys.stderr)
        return 1
    allowlist_growth = (
        sorted(set(allowed_names) - set(baseline)) if baseline is not None else []
    )
    if unknown or stale or allowlist_growth:
        print(
            render_failure(inventory, unknown, stale, allowlist_growth),
            file=sys.stderr,
        )
        return 1
    print(
        f"Cargo dependency inventory matches {len(allowed_names)} approved names "
        f"across {len(manifests)} manifests."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
