"""Closed, package-local source selectors for the native Python build tool.

The neutral fixture is not a runtime dependency. A checked-in byte-identical
package resource is loaded once, then the production collector consults these
exact seven roles. Neither a suffix nor a scoped rule grants permission to
reopen a generated or linked directory.
"""

from __future__ import annotations

import hashlib
import json
from functools import lru_cache
from importlib.resources import files
from pathlib import Path
from typing import Any

_DOMAIN = b"coding-adventures/build-tool-language-source-input-registry/v1\x00"


@lru_cache(maxsize=1)
def registry_json() -> bytes:
    """Return the wheel's checked source-input projection, not a repo path."""
    return (
        files("build_tool")
        .joinpath("language_source_input_registry.json")
        .read_bytes()
    )


@lru_cache(maxsize=1)
def _registry() -> dict[str, Any]:
    registry = json.loads(registry_json())
    if registry.get("schema_version") != 1:
        raise ValueError("unsupported source-input registry version")
    return registry


def registry_digest() -> str:
    """Compute the neutral domain-separated identity over canonical JSON."""
    canonical = json.dumps(
        _registry(), sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    return hashlib.sha256(
        _DOMAIN + len(canonical).to_bytes(8, "big") + canonical
    ).hexdigest()


def generated_components() -> frozenset[str]:
    return frozenset(_registry()["universal_inputs"]["generated_directory_components"])


def _registered_site_roots() -> frozenset[str]:
    typescript = next(
        entry for entry in _registry()["languages"] if entry["language"] == "typescript"
    )
    return frozenset(
        rule["package_root"]
        for rule in typescript["package_exact_inputs"]
        if rule["package_root"].startswith("code/sites/")
    )


def package_registry_identity(
    path: Path, language: str, repository_root: Path | None = None
) -> tuple[str, str]:
    """Check the lane encoded by a package root before file enumeration.

    Discovery supplies an explicit repository root. Isolated package trees can
    use global selectors without one, but exact-package registrations require
    the full anchored ``code/...`` spelling. A suffix alone grants no authority.
    """
    if repository_root is not None:
        try:
            parts = path.relative_to(repository_root).parts
        except ValueError as error:
            raise ValueError("package root is outside repository") from error
    else:
        all_parts = path.parts
        code_positions = [
            i for i, component in enumerate(all_parts) if component == "code"
        ]
        parts = (
            all_parts[code_positions[0]:]
            if len(code_positions) == 1
            else ()
        )

    if len(parts) == 3 and parts[:2] == ("code", "sites"):
        if repository_root is None:
            raise ValueError("site package requires an anchored repository root")
        root = "/".join(parts)
        if root not in _registered_site_roots() or language not in {
            "unknown", "typescript"
        }:
            raise ValueError("unregistered or mismatched site package root")
        # Discovery still names these two legacy graph nodes unknown/*,
        # but their reviewed build surfaces are exactly TypeScript.
        return "typescript", root
    if language not in {entry["language"] for entry in _registry()["languages"]}:
        raise ValueError(f"unknown source-input language: {language}")
    if len(parts) >= 3 and parts[:2] == ("code", "sites"):
        raise ValueError("unregistered or noncanonical site package root")
    if len(parts) >= 4 and parts[0] == "code" and parts[1] in {"packages", "programs"}:
        if parts[2] != language:
            raise ValueError(
                "package root language does not match source-input language"
            )
        return language, "/".join(parts)
    # Generic isolated test trees keep global selectors, never exact-package
    # rules. Production discovery supplies the repository root above ``code``.
    if repository_root is not None:
        raise ValueError("package root is not in a canonical code lane")
    return language, ""


def source_input_selected(
    language: str, package_root: str, relative_path: str, *, declared: bool
) -> bool:
    """Match all seven roles with fixed inputs preceding mode-specific ones."""
    registry = _registry()
    entry = next(
        (value for value in registry["languages"] if value["language"] == language),
        None,
    )
    if entry is None:
        raise ValueError(f"unknown source-input language: {language}")
    filename = relative_path.rsplit("/", 1)[-1]
    at_root = "/" not in relative_path
    universal = registry["universal_inputs"]

    if filename in universal["build_filenames"]:
        return True
    if at_root and filename in universal["root_exact_basenames"]:
        return True
    if at_root and filename in entry["root_exact_basenames"]:
        return True
    if at_root and any(
        filename.endswith(suffix) for suffix in entry["root_variable_suffixes"]
    ):
        return True
    if relative_path in entry["root_exact_relative_paths"]:
        return True
    if any(
        rule["package_root"] == package_root and relative_path in rule["paths"]
        for rule in entry["package_exact_inputs"]
    ):
        return True
    if declared:
        return False
    if any(filename.endswith(suffix) for suffix in entry["recursive_suffixes"]):
        return True
    if filename in entry["recursive_exact_basenames"]:
        return True
    for rule in entry["scoped_inputs"]:
        if rule["scope"] == "root":
            if not at_root:
                continue
        elif not relative_path.startswith(rule["path_prefix"] + "/"):
            continue
        if filename in rule["exact_basenames"] or any(
            filename.endswith(suffix) for suffix in rule["suffixes"]
        ):
            return True
    return False
