"""Validation helpers for BUILD/CI contract checks."""

from __future__ import annotations

import json
import re
from collections.abc import Iterable
from pathlib import Path
from typing import Literal, TypedDict

from build_tool import tracked_artifact_unicode17 as tracked_unicode
from build_tool.discovery import Package

CI_MANAGED_TOOLCHAIN_LANGUAGES = frozenset(
    {
        "python",
        "ruby",
        "typescript",
        "rust",
        "elixir",
        "lua",
        "perl",
        "java",
        "kotlin",
        "haskell",
    }
)
TRACKED_ARTIFACT_COMPONENT_IDENTITY = "node_modules"
TRACKED_ARTIFACT_REDACTED_PATH = "repository"
TRACKED_ARTIFACT_UNICODE_VERSION = tracked_unicode.UNICODE_VERSION
ORPHAN_SCAN_ROOT = "code"
ORPHAN_LEDGER_PATH = "code/BUILD-EXEMPTIONS"
ORPHAN_BUILD_NAMES = (
    "BUILD",
    "BUILD_windows",
    "BUILD_mac",
    "BUILD_linux",
    "BUILD_mac_and_linux",
)
ORPHAN_SKIP_COMPONENTS = frozenset(
    {
        ".git",
        "target",
        "node_modules",
        "vendor",
        ".venv",
        "_build",
        "deps",
        ".build",
        "dist-newstyle",
        ".cargo",
    }
)
WINDOWS_RESERVED_BASENAMES = frozenset(
    {
        "CON",
        "PRN",
        "AUX",
        "NUL",
        "CONIN$",
        "CONOUT$",
        "CLOCK$",
        *(f"COM{index}" for index in range(1, 10)),
        *(f"LPT{index}" for index in range(1, 10)),
        *(f"COM{index}" for index in "¹²³"),
        *(f"LPT{index}" for index in "¹²³"),
    }
)


class TrackedArtifactEntry(TypedDict):
    """One inert tracked-path record supplied by a reviewed native adapter."""

    ordinal: int
    path: str
    entry_kind: Literal["regular", "symlink", "reparse"]


class OrphanManifest(TypedDict):
    """One normalized Cargo manifest directory in a closed snapshot."""

    path: str
    kind: Literal["package", "virtual_workspace"]


class OrphanBuildFile(TypedDict):
    """One recognized BUILD path and its independently derived state."""

    path: str
    state: Literal["runnable", "empty"]


class OrphanExemption(TypedDict):
    """One inert ledger entry, including invalid kinds for fail-closed tests."""

    line: int
    kind: str
    path: str
    reason: str


class OrphanCrateSnapshot(TypedDict):
    """The complete bounded input needed by orphan-crate validation."""

    directories: list[str]
    manifests: list[OrphanManifest]
    build_files: list[OrphanBuildFile]
    exemptions: list[OrphanExemption]


class ValidationDiagnostic(TypedDict):
    """Stable language-neutral build validation diagnostic."""

    code: str
    severity: Literal["error"]
    path: str
    details: dict[str, int | str]


class OrphanCrateValidationResult(TypedDict):
    """Canonical result derived from one closed orphan-crate snapshot."""

    valid: bool
    diagnostic_codes: list[str]
    pending_exemption_count: int
    diagnostics: list[ValidationDiagnostic]


def validate_ci_full_build_toolchains(
    root: Path,
    packages: Iterable[Package],
) -> str | None:
    """Return an error message when CI full-build toolchains drift."""
    ci_path = root / ".github" / "workflows" / "ci.yml"
    try:
        workflow = ci_path.read_text(encoding="utf-8")
    except OSError:
        return None

    if "Full build on main merge" not in workflow:
        return None

    compact_workflow = "".join(workflow.split())
    missing_output_binding: list[str] = []
    missing_main_force: list[str] = []

    for lang in _languages_needing_ci_toolchains(packages):
        output_binding = (
            f"needs_{lang}:${{{{steps.toolchains.outputs.needs_{lang}}}}}"
        )
        if output_binding not in compact_workflow:
            missing_output_binding.append(lang)

        if f"needs_{lang}=true" not in compact_workflow:
            missing_main_force.append(lang)

    if not missing_output_binding and not missing_main_force:
        return None

    parts: list[str] = []
    if missing_output_binding:
        parts.append(
            "detect outputs for forced main full builds are not normalized through "
            f"steps.toolchains for: {', '.join(missing_output_binding)}"
        )
    if missing_main_force:
        parts.append(
            "forced main full-build path does not explicitly enable toolchains for: "
            f"{', '.join(missing_main_force)}"
        )

    return f"{ci_path.as_posix()}: {'; '.join(parts)}"


def validate_build_contracts(
    root: Path,
    packages: Iterable[Package],
) -> str | None:
    """Return combined BUILD/CI validation failures, if any."""
    package_list = list(packages)
    errors: list[str] = []

    ci_error = validate_ci_full_build_toolchains(root, package_list)
    if ci_error is not None:
        errors.append(ci_error)

    errors.extend(validate_lua_isolated_build_files(package_list))
    errors.extend(validate_perl_build_files(package_list))

    if not errors:
        return None
    return "\n  - ".join(errors)


def validate_lua_isolated_build_files(packages: Iterable[Package]) -> list[str]:
    """Validate Lua BUILD contracts needed for isolated LuaRocks builds."""
    errors: list[str] = []

    for pkg in packages:
        if pkg.language != "lua":
            continue

        self_rock = f"coding-adventures-{pkg.path.name.replace('_', '-')}"
        build_lines: dict[str, list[str]] = {}
        for build_path in sorted(pkg.path.glob("BUILD*")):
            lines = _read_build_lines(build_path)
            build_lines[build_path.name] = lines
            if not lines:
                continue

            foreign_remove = _first_foreign_lua_remove(lines, self_rock)
            if foreign_remove is not None:
                errors.append(
                    f"{build_path.as_posix()}: Lua BUILD removes unrelated rock "
                    f"{foreign_remove}; isolated package builds should only remove "
                    "the package they are rebuilding"
                )

            state_machine_index = _first_line_containing(
                lines, ("../state_machine", "..\\state_machine")
            )
            directed_graph_index = _first_line_containing(
                lines, ("../directed_graph", "..\\directed_graph")
            )
            if (
                state_machine_index is not None
                and directed_graph_index is not None
                and state_machine_index < directed_graph_index
            ):
                errors.append(
                    f"{build_path.as_posix()}: Lua BUILD installs state_machine "
                    "before directed_graph; isolated LuaRocks builds require "
                    "directed_graph first"
                )

            if (
                _has_guarded_local_lua_install(lines)
                or (
                    build_path.name == "BUILD_windows"
                    and _has_local_lua_sibling_install(lines)
                )
            ) and not _self_install_disables_deps(lines, self_rock):
                errors.append(
                    f"{build_path.as_posix()}: Lua BUILD bootstraps sibling rocks "
                    "but the final self-install does not pass "
                    "--deps-mode=none or --no-manifest"
                )

        missing_windows_deps = _missing_lua_sibling_installs(
            build_lines.get("BUILD", []),
            build_lines.get("BUILD_windows", []),
        )
        if missing_windows_deps:
            errors.append(
                f"{(pkg.path / 'BUILD_windows').as_posix()}: Lua BUILD_windows is "
                "missing sibling installs present in BUILD: "
                f"{', '.join(missing_windows_deps)}"
            )

    return errors


def validate_perl_build_files(packages: Iterable[Package]) -> list[str]:
    """Validate Perl BUILD contracts needed for isolated cpanm installs."""
    errors: list[str] = []

    for pkg in packages:
        if pkg.language != "perl":
            continue

        for build_path in sorted(pkg.path.glob("BUILD*")):
            for line in _read_build_lines(build_path):
                if (
                    "cpanm" in line
                    and "Test2::V0" in line
                    and "--notest" not in line
                ):
                    errors.append(
                        f"{build_path.as_posix()}: Perl BUILD bootstraps "
                        "Test2::V0 without --notest; isolated Windows installs "
                        "can fail while installing the test framework itself"
                    )
                    break

    return errors


def validate_tracked_artifact_snapshot(
    entries: Iterable[TrackedArtifactEntry],
    *,
    unicode_version: str = TRACKED_ARTIFACT_UNICODE_VERSION,
) -> list[ValidationDiagnostic]:
    """Derive tracked-artifact diagnostics from inert path records only."""
    if unicode_version != TRACKED_ARTIFACT_UNICODE_VERSION:
        raise ValueError(
            "tracked artifact Unicode version must be "
            f"{TRACKED_ARTIFACT_UNICODE_VERSION}"
        )
    diagnostics: list[ValidationDiagnostic] = []

    for entry in entries:
        normalized_path, problem = _normalize_tracked_artifact_path(entry["path"])
        details: dict[str, int | str] = {
            "ordinal": entry["ordinal"],
            "entry_kind": entry["entry_kind"],
        }
        if problem is not None:
            details["problem"] = problem
            diagnostics.append(
                {
                    "code": "TRACKED_ARTIFACT_PATH_INVALID",
                    "severity": "error",
                    "path": TRACKED_ARTIFACT_REDACTED_PATH,
                    "details": details,
                }
            )
            continue

        if normalized_path is None:
            raise AssertionError("valid tracked artifact path did not normalize")
        if any(
            tracked_unicode.nfkc_casefold(component)
            == TRACKED_ARTIFACT_COMPONENT_IDENTITY
            for component in normalized_path.split("/")
        ):
            diagnostics.append(
                {
                    "code": "TRACKED_ARTIFACT_FORBIDDEN",
                    "severity": "error",
                    "path": normalized_path,
                    "details": details,
                }
            )

    return sorted(
        diagnostics,
        key=lambda item: (
            item["code"],
            item["path"],
            json.dumps(item["details"], sort_keys=True),
        ),
    )


def validate_orphan_crate_snapshot(
    snapshot: OrphanCrateSnapshot,
) -> OrphanCrateValidationResult:
    """Validate inert Cargo, BUILD, and exemption records without host authority.

    Snapshot construction is deliberately outside this function. The validator
    does not enumerate a checkout, inspect Git, open a path, launch a process,
    read the environment, or access the network.
    """
    manifests = [
        manifest
        for manifest in snapshot["manifests"]
        if not _is_orphan_artifact_path(manifest["path"])
    ]
    directories = set(snapshot["directories"])
    manifest_by_path = {manifest["path"]: manifest for manifest in manifests}
    coverage = {
        manifest["path"]: _find_covering_build(
            snapshot["build_files"], manifest["path"], "runnable"
        )
        for manifest in manifests
    }
    empty_builds = {
        manifest["path"]: _find_covering_build(
            snapshot["build_files"], manifest["path"], "empty"
        )
        for manifest in manifests
    }

    diagnostics: list[ValidationDiagnostic] = []
    seen_exemption_paths: set[str] = set()
    valid_exemptions: list[OrphanExemption] = []

    # Reserve every portable identity before applying the policy-field
    # precedence. Thus an invalid first spelling cannot hide a later alias.
    for exemption in snapshot["exemptions"]:
        path = exemption["path"]
        identity: str | None = None
        path_problem: str | None = None
        if not _is_portable_orphan_path(path):
            path_problem = "PATH_UNSAFE"
        else:
            identity = _orphan_path_identity(path)
            if not _is_under_orphan_scan_root(path):
                path_problem = "PATH_OUTSIDE_SCAN"
            elif _is_orphan_artifact_path(path):
                path_problem = "PATH_ARTIFACT"

        duplicate = identity is not None and identity in seen_exemption_paths
        if identity is not None and not duplicate:
            seen_exemption_paths.add(identity)

        problem: str | None
        if exemption["kind"] not in {"EXCLUDED", "PENDING"}:
            problem = "UNKNOWN_KIND"
        elif not exemption["reason"].strip():
            problem = "REASON_MISSING"
        elif duplicate:
            problem = "DUPLICATE_PATH"
        else:
            problem = path_problem

        if problem is not None:
            diagnostics.append(
                {
                    "code": "ORPHAN_EXEMPTION_INVALID",
                    "severity": "error",
                    "path": ORPHAN_LEDGER_PATH,
                    "details": {"line": exemption["line"], "problem": problem},
                }
            )
            continue
        valid_exemptions.append(exemption)

    active_exemptions: dict[str, OrphanExemption] = {}
    pending_exemption_count = 0
    for exemption in valid_exemptions:
        exemption_path = exemption["path"]
        stale_problem: str | None = None
        if exemption_path not in directories:
            stale_problem = "MISSING_DIRECTORY"
        elif exemption_path not in manifest_by_path:
            stale_problem = "NO_MANIFEST"
        elif coverage[exemption_path] is not None:
            stale_problem = "COVERED"

        if stale_problem is not None:
            diagnostics.append(
                {
                    "code": "ORPHAN_EXEMPTION_STALE",
                    "severity": "error",
                    "path": ORPHAN_LEDGER_PATH,
                    "details": {
                        "entry_path": exemption_path,
                        "kind": exemption["kind"],
                        "line": exemption["line"],
                        "problem": stale_problem,
                    },
                }
            )
            continue

        active_exemptions[exemption_path] = exemption
        if exemption["kind"] == "PENDING":
            pending_exemption_count += 1

    for manifest in manifests:
        manifest_path = manifest["path"]
        if coverage[manifest_path] is not None or manifest_path in active_exemptions:
            continue
        empty_build = empty_builds[manifest_path]
        if empty_build is None:
            diagnostics.append(
                {
                    "code": "ORPHAN_CRATE_UNLISTED",
                    "severity": "error",
                    "path": manifest_path,
                    "details": {"manifest_kind": manifest["kind"]},
                }
            )
        else:
            diagnostics.append(
                {
                    "code": "ORPHAN_CRATE_EMPTY_BUILD",
                    "severity": "error",
                    "path": manifest_path,
                    "details": {
                        "build_path": empty_build["path"],
                        "manifest_kind": manifest["kind"],
                    },
                }
            )

    diagnostics.sort(key=_validation_diagnostic_sort_key)
    diagnostic_codes = sorted({diagnostic["code"] for diagnostic in diagnostics})
    return {
        "valid": not diagnostics,
        "diagnostic_codes": diagnostic_codes,
        "pending_exemption_count": pending_exemption_count,
        "diagnostics": diagnostics,
    }


def _find_covering_build(
    build_files: Iterable[OrphanBuildFile],
    manifest_path: str,
    state: Literal["runnable", "empty"],
) -> OrphanBuildFile | None:
    """Choose the deepest component-wise ancestor, then the fixed name rank."""
    build_name_rank = {name: index for index, name in enumerate(ORPHAN_BUILD_NAMES)}
    candidates: list[OrphanBuildFile] = []
    for build_file in build_files:
        if build_file["state"] != state:
            continue
        parent, _, name = build_file["path"].rpartition("/")
        if not _is_under_orphan_scan_root(parent):
            continue
        if manifest_path != parent and not manifest_path.startswith(f"{parent}/"):
            continue
        if name not in build_name_rank:
            continue
        candidates.append(build_file)

    if not candidates:
        return None
    return min(
        candidates,
        key=lambda item: (
            -len(item["path"].rpartition("/")[0].split("/")),
            build_name_rank[item["path"].rpartition("/")[2]],
            item["path"],
        ),
    )


def _is_portable_orphan_path(path: str) -> bool:
    """Apply the shared portable directory grammar without host path APIs."""
    if not path or len(path) > 512 or tracked_unicode.nfc(path) != path:
        return False
    if path.startswith("/") or "\\" in path or "//" in path:
        return False
    if len(path) >= 2 and path[0].isascii() and path[0].isalpha() and path[1] == ":":
        return False
    if any(ord(character) < 32 or character in '<>:"|?*' for character in path):
        return False
    for component in path.split("/"):
        if not component or component in {".", ".."}:
            return False
        if component.endswith((" ", ".")):
            return False
        basename = tracked_unicode.full_uppercase(component.split(".", 1)[0])
        if basename in WINDOWS_RESERVED_BASENAMES:
            return False
    return True


def _orphan_path_identity(path: str) -> str:
    return tracked_unicode.casefold(tracked_unicode.nfc(path))


def _is_under_orphan_scan_root(path: str) -> bool:
    return path == ORPHAN_SCAN_ROOT or path.startswith(f"{ORPHAN_SCAN_ROOT}/")


def _is_orphan_artifact_path(path: str) -> bool:
    return any(component in ORPHAN_SKIP_COMPONENTS for component in path.split("/"))


def _validation_diagnostic_sort_key(
    diagnostic: ValidationDiagnostic,
) -> tuple[str, str, str, str]:
    return (
        diagnostic["code"],
        diagnostic["path"],
        "",
        json.dumps(diagnostic["details"], sort_keys=True),
    )


def _languages_needing_ci_toolchains(packages: Iterable[Package]) -> list[str]:
    return sorted(
        {
            pkg.language
            for pkg in packages
            if pkg.language in CI_MANAGED_TOOLCHAIN_LANGUAGES
        }
    )


def _normalize_tracked_artifact_path(path: str) -> tuple[str | None, str | None]:
    normalized = path.replace("\\", "/")
    if not normalized:
        return None, "EMPTY"
    if len(normalized) > 512:
        return None, "TOO_LONG"
    if normalized != tracked_unicode.nfc(normalized):
        return None, "NON_NFC"
    if normalized.startswith("/"):
        return None, "ABSOLUTE"
    if re.match(r"^[A-Za-z]:", normalized):
        return None, "DRIVE_QUALIFIED"
    if any(not segment for segment in normalized.split("/")):
        return None, "EMPTY_SEGMENT"
    if any(ord(character) < 32 or character in '<>:"|?*' for character in normalized):
        return None, "UNSAFE_CHARACTER"
    for segment in normalized.split("/"):
        if segment in {".", ".."}:
            return None, "DOT_SEGMENT"
        if segment.endswith((" ", ".")):
            return None, "TRAILING_DOT_OR_SPACE"
        basename = tracked_unicode.full_uppercase(segment.split(".", 1)[0])
        if basename in WINDOWS_RESERVED_BASENAMES:
            return None, "RESERVED_BASENAME"
    return normalized, None


def _read_build_lines(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return []

    return [
        line.strip()
        for line in text.splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]


def _first_foreign_lua_remove(lines: Iterable[str], self_rock: str) -> str | None:
    pattern = re.compile(r"\bluarocks remove --force ([^ \t]+)")
    for line in lines:
        match = pattern.search(line)
        if match is None:
            continue
        if match.group(1) != self_rock:
            return match.group(1)
    return None


def _first_line_containing(lines: list[str], needles: tuple[str, ...]) -> int | None:
    for index, line in enumerate(lines):
        if any(needle in line for needle in needles):
            return index
    return None


def _has_guarded_local_lua_install(lines: Iterable[str]) -> bool:
    return any(
        "luarocks show " in line and ("../" in line or "..\\" in line)
        for line in lines
    )


def _has_local_lua_sibling_install(lines: Iterable[str]) -> bool:
    return bool(_lua_sibling_install_dirs(lines))


def _self_install_disables_deps(lines: Iterable[str], self_rock: str) -> bool:
    for line in lines:
        if "luarocks make" not in line or self_rock not in line:
            continue
        if (
            "--deps-mode=none" in line
            or "--deps-mode none" in line
            or "--no-manifest" in line
        ):
            return True
    return False


def _missing_lua_sibling_installs(
    unix_lines: Iterable[str],
    windows_lines: Iterable[str],
) -> list[str]:
    unix_deps = _lua_sibling_install_dirs(unix_lines)
    windows_deps = set(_lua_sibling_install_dirs(windows_lines))
    return [dep for dep in unix_deps if dep not in windows_deps]


def _lua_sibling_install_dirs(lines: Iterable[str]) -> list[str]:
    deps: set[str] = set()
    pattern = re.compile(r"\bcd\s+([.][.][\\/][^ \t\r\n&()]+)")

    for line in lines:
        if "luarocks make" not in line:
            continue
        match = pattern.search(line)
        if match is None:
            continue
        deps.add(match.group(1).replace("\\", "/"))

    return sorted(deps)
